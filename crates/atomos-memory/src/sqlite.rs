use crate::{key, text, Error, Fact, Identity, MemoryStore, Result, Retrieval, Value};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, Row, TransactionBehavior};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use crate::{ContextRequest, RankedRetrieval};
use crate::retrieval::{rank, score, MAX_CANDIDATES};
use std::fs::{self, DirBuilder, OpenOptions};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::Path;
use std::time::{Duration, Instant};

mod recovery;
const SCHEMA: &str = include_str!("schema.sql");
const POLICY: &str = include_str!("voice-policy.sql");
const VERSION: i64 = 2;
const APP_ID: i64 = 0x414d4731;
const MAX_RESULTS: usize = 50;
const MAX_PAYLOAD: usize = 16384;
const MAX_VISITED: usize = 256;
const FANOUT: usize = 32;
// 128 MiB main file (4 KiB pages); reserve journal headroom. Not a
// certified aggregate 256 MiB quota, nor a replacement for a filesystem quota.
const PAGE_CAP: i64 = 32768;
const SELECT_FACT: &str = "SELECT a.id,s.entity_key,a.predicate,a.value_kind,
 a.text_value,a.int_value,t.entity_key,a.source_id,a.recorded_at,a.superseded_at,a.supersedes_id
 FROM assertion a JOIN entity s ON s.id=a.subject_id AND s.owner_scope=a.owner_scope
 LEFT JOIN entity t ON t.id=a.target_id AND t.owner_scope=a.owner_scope";

pub struct SqliteMemory {
    conn: Connection,
    identity: Identity,
}
fn private_dir(dir: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(dir)?;
    if !meta.is_dir() || meta.mode() & 0o077 != 0 {
        return Err(Error::Invalid("AMG requires a private non-symlink directory (0700)"));
    }
    Ok(())
}
fn private_file(path: &Path) -> Result<()> {
    let m = fs::symlink_metadata(path)?;
    if !m.is_file() || m.mode() & 0o077 != 0 || m.nlink() != 1 {
        return Err(Error::Invalid("AMG requires a private regular database file (0600)"));
    }
    Ok(())
}
fn now(c: &Connection) -> rusqlite::Result<i64> {
    c.query_row("SELECT unixepoch()", [], |r| r.get(0))
}
fn connect(path: &Path) -> Result<Connection> {
    private_file(path)?;
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    conn.busy_timeout(Duration::from_millis(250))?;
    Ok(conn)
}
fn configure(c: &Connection) -> Result<()> {
    c.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL; PRAGMA cache_size=-2048; PRAGMA secure_delete=ON;")?;
    let mode: String = c.query_row("PRAGMA journal_mode=DELETE", [], |r| r.get(0))?;
    let size: i64 = c.query_row("PRAGMA page_size", [], |r| r.get(0))?;
    let cap: i64 = c.query_row("PRAGMA max_page_count=32768", [], |r| r.get(0))?;
    if mode != "delete" || size != 4096 || cap > PAGE_CAP {
        return Err(Error::Invalid("unsupported SQLite journal/page configuration or database exceeds cap"));
    }
    Ok(())
}
fn entity_id(c: &Connection, owner: &str, k: &str) -> Result<i64> {
    key(k)?;
    c.query_row("SELECT id FROM entity WHERE owner_scope=?1 AND entity_key=?2",
        params![owner,k], |r| r.get(0)).optional()?.ok_or(Error::Missing)
}
fn row_fact(r: &Row<'_>) -> rusqlite::Result<Fact> {
    let kind: String = r.get(3)?;
    let value = match kind.as_str() {
        "text" => Value::Text(r.get(4)?),
        "integer" => Value::Integer(r.get(5)?),
        "boolean" => Value::Boolean(r.get::<_,i64>(5)? != 0),
        "entity" => Value::Entity(r.get(6)?),
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    Ok(Fact { revision:r.get(0)?, subject:r.get(1)?, predicate:r.get(2)?,
        value, source_id:r.get(7)?, recorded_at:r.get(8)?, superseded_at:r.get(9)?, supersedes:r.get(10)? })
}
fn current(c: &Connection, owner: &str, subject: i64, predicate: &str) -> Result<Option<Fact>> {
    let sql = format!("{SELECT_FACT} WHERE a.owner_scope=?1 AND a.subject_id=?2 AND a.predicate=?3 AND a.superseded_at IS NULL");
    Ok(c.query_row(&sql, params![owner,subject,predicate], row_fact).optional()?)
}
fn payload(f: &Fact) -> usize {
    f.subject.len() + f.predicate.len() + 128 + match &f.value {
        Value::Text(v)|Value::Entity(v) => v.len(), _ => 32,
    }
}
fn collect(mut rows: rusqlite::Rows<'_>) -> Result<Retrieval> {
    let mut out = Retrieval { facts: Vec::new(), truncated:false, visited:0 };
    let mut bytes = 0;
    while let Some(row) = rows.next()? {
        let f = row_fact(row)?;
        if out.facts.len() >= MAX_RESULTS || bytes + payload(&f) > MAX_PAYLOAD {
            out.truncated=true; break;
        }
        bytes += payload(&f);
        out.facts.push(f);
    }
    Ok(out)
}

impl SqliteMemory {
    pub fn initialize(dir: &Path, owner: &str, name: &str) -> Result<Self> {
        key(owner)?; text(name,128)?;
        // Explicit init only. Never overwrite or silently recreate a store.
        if !dir.exists() { DirBuilder::new().mode(0o700).create(dir)?; }
        private_dir(dir)?;
        let path = dir.join("memory.db");
        OpenOptions::new().write(true).create_new(true).mode(0o600).open(&path)?;
        let mut conn = connect(&path)?;
        conn.execute_batch("PRAGMA page_size=4096;")?;
        configure(&conn)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(SCHEMA)?;
        tx.execute_batch(POLICY)?;
        tx.execute("INSERT INTO identity VALUES(1,lower(hex(randomblob(16))),?1,?2,unixepoch())", params![owner,name])?;
        tx.execute("INSERT INTO entity(owner_scope,entity_key,label,kind) VALUES(?1,'owner',?1,'person')", [owner])?;
        tx.pragma_update(None,"application_id", APP_ID)?;
        tx.pragma_update(None,"user_version", VERSION)?;
        tx.commit()?;
        drop(conn);
        Self::open(dir,owner)
    }
    pub fn open(dir: &Path, owner: &str) -> Result<Self> {
        key(owner)?; private_dir(dir)?;
        let conn = connect(&dir.join("memory.db"))?;
        let app: i64 = conn.query_row("PRAGMA application_id",[],|r| r.get(0))?;
        let version: i64 = conn.query_row("PRAGMA user_version",[],|r| r.get(0))?;
        if app != APP_ID || version != VERSION { return Err(Error::UnsupportedSchema); }
        let identity = conn.query_row("SELECT agent_id,owner_scope,display_name,created_at FROM identity WHERE singleton=1",
            [], |r| Ok(Identity { agent_id:r.get(0)?,owner_scope:r.get(1)?,display_name:r.get(2)?,created_at:r.get(3)? }))?;
        if identity.owner_scope != owner { return Err(Error::WrongOwner); }
        configure(&conn)?;
        // Resolve the virtual table at startup, rather than silently losing search.
        conn.query_row("SELECT count(*) FROM fact_search WHERE rowid=-1",[],|r| r.get::<_,i64>(0))?;
        Ok(Self { conn, identity })
    }
    fn write_fact(&mut self, subject: &str, predicate: &str, expected: Option<i64>, value: Value) -> Result<Fact> {
        key(subject)?; key(predicate)?;
        if let Value::Text(s) = &value { text(s,1024)?; }
        let owner = &self.identity.owner_scope;
        let tx = self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let sid = entity_id(&tx,owner,subject)?;
        let old = current(&tx,owner,sid,predicate)?;
        if old.as_ref().map(|f| f.revision) != expected { return Err(Error::Conflict); }
        let (kind, t, i, target) = match &value {
            Value::Text(v) => ("text",Some(v.as_str()),None,None),
            Value::Integer(v) => ("integer",None,Some(*v),None),
            Value::Boolean(v) => ("boolean",None,Some(i64::from(*v)),None),
            Value::Entity(v) => ("entity",None,None,Some(entity_id(&tx,owner,v)?)),
        };
        let time = now(&tx)?;
        if let Some(id) = expected {
            tx.execute("UPDATE assertion SET superseded_at=?1 WHERE id=?2 AND owner_scope=?3",params![time,id,owner])?;
        }
        tx.execute("INSERT INTO source(owner_scope,kind,recorded_at) VALUES(?1,'owner_local',?2)",params![owner,time])?;
        let source = tx.last_insert_rowid();
        tx.execute("INSERT INTO assertion(owner_scope,subject_id,predicate,value_kind,text_value,int_value,target_id,source_id,recorded_at,supersedes_id)
            VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![owner,sid,predicate,kind,t,i,target,source,time,expected])?;
        let result = Fact { revision:tx.last_insert_rowid(),subject:subject.into(),predicate:predicate.into(),
            value,source_id:source,recorded_at:time,superseded_at:None,supersedes:expected };
        tx.commit()?;
        Ok(result)
    }
}

impl MemoryStore for SqliteMemory {
    fn identity(&self) -> &Identity { &self.identity }
    fn add_entity(&mut self, k: &str, label: &str, kind: &str) -> Result<()> {
        key(k)?; key(kind)?; text(label,128)?;
        let n = self.conn.execute("INSERT INTO entity(owner_scope,entity_key,label,kind) VALUES(?1,?2,?3,?4)
          ON CONFLICT(owner_scope,entity_key) DO NOTHING",params![self.identity.owner_scope,k,label,kind])?;
        if n != 1 { return Err(Error::Conflict); }
        Ok(())
    }
    fn remember(&mut self, s:&str, p:&str, v:Value) -> Result<Fact> { self.write_fact(s,p,None,v) }
    fn correct(&mut self, s:&str,p:&str,expected:i64,v:Value) -> Result<Fact> { self.write_fact(s,p,Some(expected),v) }
    fn recall(&self, s:&str,p:&str) -> Result<Option<Fact>> {
        key(p)?;
        let o=&self.identity.owner_scope;
        current(&self.conn,o,entity_id(&self.conn,o,s)?,p)
    }
    fn forget(&mut self,s:&str,p:&str) -> Result<usize> {
        key(p)?;
        let o=&self.identity.owner_scope;
        let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let sid=entity_id(&tx,o,s)?;
        let n=tx.execute("DELETE FROM assertion WHERE owner_scope=?1 AND subject_id=?2 AND predicate=?3",params![o,sid,p])?;
        tx.execute("DELETE FROM source WHERE owner_scope=?1 AND NOT EXISTS(SELECT 1 FROM assertion WHERE assertion.source_id=source.id)",[o])?;
        tx.commit()?;
        Ok(n)
    }
    fn history(&self,s:&str,p:&str) -> Result<Retrieval> {
        key(p)?;
        let o=&self.identity.owner_scope;
        let sid=entity_id(&self.conn,o,s)?;
        let sql=format!("{SELECT_FACT} WHERE a.owner_scope=?1 AND a.subject_id=?2 AND a.predicate=?3 ORDER BY a.id DESC LIMIT 51");
        let mut stmt=self.conn.prepare(&sql)?;
        let result=collect(stmt.query(params![o,sid,p])?)?;
        Ok(result)
    }
    fn search(&self,term:&str) -> Result<Retrieval> {
        text(term,128)?;
        // One literal phrase, not caller-provided FTS syntax.
        if !term.chars().any(char::is_alphanumeric) { return Err(Error::Invalid("search requires a word")); }
        let phrase=format!("\"{}\"",term.replace('"',"\"\""));
        let sql=format!("{SELECT_FACT} JOIN fact_search ON fact_search.rowid=a.id WHERE a.owner_scope=?1 AND a.superseded_at IS NULL AND fact_search MATCH ?2 ORDER BY rank,a.id LIMIT 51");
        let mut stmt=self.conn.prepare(&sql)?;
        let result=collect(stmt.query(params![self.identity.owner_scope,phrase])?)?;
        Ok(result)
    }
    fn neighbors(&self,s:&str,depth:usize) -> Result<Retrieval> {
        if !(1..=3).contains(&depth) { return Err(Error::Invalid("graph depth must be 1–3")); }
        let o=&self.identity.owner_scope;
        let root=entity_id(&self.conn,o,s)?;
        // One read transaction provides a consistent traversal snapshot.
        let tx=self.conn.unchecked_transaction()?;
        let mut seen=BTreeSet::from([root]);
        let mut edges=BTreeSet::new();
        let mut queue=VecDeque::from([(root,0usize)]);
        let mut out=Retrieval { facts:Vec::new(),truncated:false,visited:1 };
        let mut bytes=0;
        'walk: while let Some((node,level))=queue.pop_front() {
            if level >= depth { continue; }
            for column in ["subject_id","target_id"] {
                let sql=format!("{SELECT_FACT} WHERE a.owner_scope=?1 AND a.{column}=?2 AND a.superseded_at IS NULL AND a.target_id IS NOT NULL ORDER BY a.id LIMIT 33");
                let mut stmt=tx.prepare(&sql)?;
                let mut rows=stmt.query(params![o,node])?;
                let mut count=0;
                while let Some(row)=rows.next()? {
                    count+=1;
                    if count > FANOUT { out.truncated=true; break; }
                    let f=row_fact(row)?;
                    if !edges.insert(f.revision) { continue; }
                    if out.facts.len() >= MAX_RESULTS || bytes+payload(&f)>MAX_PAYLOAD {
                        out.truncated=true; break 'walk;
                    }
                    let other=if column=="subject_id" {
                        match &f.value { Value::Entity(v)=>v.as_str(), _=>return Err(Error::Invalid("invalid edge")) }
                    } else { f.subject.as_str() };
                    let id=entity_id(&tx,o,other)?;
                    if !seen.contains(&id) {
                        if seen.len() >= MAX_VISITED { out.truncated=true; break 'walk; }
                        seen.insert(id); queue.push_back((id,level+1));
                    }
                    bytes+=payload(&f); out.facts.push(f);
                }
            }
        }
        out.visited=seen.len();
        tx.commit()?;
        Ok(out)
    }
}

/// Separate read-only SQLite connection for the conversational runtime.
/// Never initializes a store, runs a migration or applies write pragmas.
pub struct SqliteReader {
    conn: Connection,
    identity: Identity,
}
impl SqliteReader {
    pub fn open(dir: &Path, owner: &str) -> Result<Self> {
        key(owner)?;
        private_dir(dir)?;
        let path = dir.join("memory.db");
        private_file(&path)?;
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
        conn.busy_timeout(Duration::from_millis(100))?;
        conn.execute_batch("PRAGMA query_only=ON; PRAGMA cache_size=-2048;")?;
        let app: i64 = conn.query_row("PRAGMA application_id",[],|r| r.get(0))?;
        let version: i64 = conn.query_row("PRAGMA user_version",[],|r| r.get(0))?;
        if app != APP_ID || version != VERSION { return Err(Error::UnsupportedSchema); }
        let identity = conn.query_row("SELECT agent_id,owner_scope,display_name,created_at FROM identity WHERE singleton=1",
            [], |r| Ok(Identity { agent_id:r.get(0)?,owner_scope:r.get(1)?,display_name:r.get(2)?,created_at:r.get(3)? }))?;
        if identity.owner_scope != owner { return Err(Error::WrongOwner); }
        text(&identity.display_name,128)?;
        conn.query_row("SELECT count(*) FROM fact_search WHERE rowid=-1",[],|r| r.get::<_,i64>(0))?;
        Ok(Self { conn, identity })
    }
}
impl crate::MemoryReader for SqliteReader {
    fn identity(&self) -> &Identity { &self.identity }
    fn recall(&self, s:&str,p:&str) -> Result<Option<Fact>> {
        key(p)?;
        let o=&self.identity.owner_scope;
        let tx=self.conn.unchecked_transaction()?;
        let id=match entity_id(&tx,o,s) {
            Ok(id)=>id,
            Err(Error::Missing)=>return Ok(None),
            Err(e)=>return Err(e),
        };
        let fact=current(&tx,o,id,p)?;
        tx.commit()?;
        Ok(fact)
    }
    fn recall_voice(&self,s:&str,p:&str)->Result<Option<Fact>> {
        if s!="owner" || !matches!(p,"favorite_color"|"favorite_food"|"favorite_music") {
            return Err(Error::DisclosureDenied);
        }
        let o=&self.identity.owner_scope;
        let tx=self.conn.unchecked_transaction()?;
        let id=entity_id(&tx,o,s)?;
        let fact=current(&tx,o,id,p)?;
        if let Some(f)=&fact {
            if !matches!(&f.value,Value::Text(_)) { return Err(Error::DisclosureDenied); }
            let allowed:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM voice_grant WHERE assertion_id=?1 AND expires_at>unixepoch())",[f.revision],|r|r.get(0))?;
            if !allowed { return Err(Error::DisclosureDenied); }
        }
        tx.commit()?;
        Ok(fact)
    }
    fn retrieve(&self, request:&ContextRequest) -> Result<RankedRetrieval> {
        let terms=request.validate()?;
        // A single deadline covers the snapshot and all its SQL statements.
        // SQLite calls the hook periodically; lock waits are separately bounded.
        let deadline=Instant::now()+Duration::from_millis(250);
        let mut ticks=0u32;
        self.conn.progress_handler(1000,Some(move || {
            ticks+=1;
            ticks>=2000 || Instant::now()>=deadline
        }));
        let result=self.retrieve_snapshot(request,&terms,deadline);
        self.conn.progress_handler(0,None::<fn()->bool>);
        result
    }
}


// Narrower initial context work cap than the generic graph administration
// API. Paths are retained only inside this transaction; nothing is cached.
const CONTEXT_NODES: usize=32;
const CONTEXT_FANOUT: usize=32;
impl SqliteReader {
    fn retrieve_snapshot(&self, q:&ContextRequest, terms:&[String], deadline:Instant) -> Result<RankedRetrieval> {
        let o=&self.identity.owner_scope;
        let tx=self.conn.unchecked_transaction()?;
        let root=entity_id(&tx,o,&q.anchor)?;
        let mut required=Vec::new();
        let mut missing=Vec::new();
        for p in &q.required {
            match current(&tx,o,root,p)? {
                Some(f)=>required.push(f),
                None=>missing.push(p.clone()),
            }
        }
        let mut paths:BTreeMap<i64,Vec<Fact>>=BTreeMap::from([(root,Vec::new())]);
        let mut queue=VecDeque::from([root]);
        let mut incomplete=false;
        while let Some(node)=queue.pop_front() {
            if Instant::now()>=deadline { return Err(Error::Invalid("context retrieval deadline exceeded")); }
            let path=paths.get(&node).ok_or(Error::Missing)?.clone();
            if path.len()>=q.depth { continue; }
            for column in ["subject_id","target_id"] {
                // Fixed identifier alternatives; all external data is bound.
                let sql=format!("{SELECT_FACT} WHERE a.owner_scope=?1 AND a.{column}=?2 AND a.superseded_at IS NULL AND a.target_id IS NOT NULL ORDER BY a.id LIMIT 33");
                let mut stmt=tx.prepare(&sql)?;
                let mut rows=stmt.query(params![o,node])?;
                let mut count=0;
                while let Some(row)=rows.next()? {
                    count+=1;
                    if count>CONTEXT_FANOUT { incomplete=true; break; }
                    let edge=row_fact(row)?;
                    let other=if column=="subject_id" {
                        match &edge.value { Value::Entity(v)=>v.as_str(), _=>return Err(Error::Invalid("invalid graph edge")) }
                    } else { edge.subject.as_str() };
                    let id=entity_id(&tx,o,other)?;
                    if paths.contains_key(&id) { continue; }
                    if paths.len()>=CONTEXT_NODES { incomplete=true; continue; }
                    let mut next=path.clone();
                    next.push(edge);
                    paths.insert(id,next);
                    queue.push_back(id);
                }
            }
        }
        let mut matches=Vec::new();
        if !terms.is_empty() {
            // Terms contain only alphanumeric characters. Quoted OR terms are
            // generated by the application, never raw FTS syntax.
            let expression=terms.iter().map(|t|format!("\"{t}\"")).collect::<Vec<_>>().join(" OR ");
            let markers=std::iter::repeat("?").take(paths.len()).collect::<Vec<_>>().join(",");
            let query=format!("{SELECT_FACT} JOIN fact_search ON fact_search.rowid=a.id
                WHERE a.owner_scope=? AND a.superseded_at IS NULL AND fact_search MATCH ?
                  AND a.subject_id IN ({markers}) ORDER BY rank,a.id LIMIT 51");
            let mut bindings=vec![rusqlite::types::Value::Text(o.clone()),rusqlite::types::Value::Text(expression)];
            bindings.extend(paths.keys().copied().map(rusqlite::types::Value::Integer));
            let mut stmt=tx.prepare(&query)?;
            let mut rows=stmt.query(rusqlite::params_from_iter(bindings.iter()))?;
            let mut bytes=0usize;
            while let Some(row)=rows.next()? {
                if Instant::now()>=deadline { return Err(Error::Invalid("context retrieval deadline exceeded")); }
                let f=row_fact(row)?;
                if matches.len()>=MAX_CANDIDATES || bytes+payload(&f)>MAX_PAYLOAD {
                    incomplete=true; break;
                }
                let id=entity_id(&tx,o,&f.subject)?;
                let path=paths.get(&id).ok_or(Error::Missing)?;
                let rank_score=score(&f,terms,path.len(),matches.len());
                bytes+=payload(&f);
                matches.push((f,rank_score,path.clone()));
            }
        }
        if Instant::now()>=deadline { return Err(Error::Invalid("context retrieval deadline exceeded")); }
        let out=rank(matches,required,missing,paths.len(),incomplete,q.max_facts);
        tx.commit()?;
        Ok(out)
    }
}


impl SqliteMemory {
    /// Explicit migration only. The v1 database remains intact on failure.
    /// All old facts default to local text; no voice grants are inherited.
    pub fn migrate(dir:&Path, owner:&str)->Result<()> {
        key(owner)?;
        private_dir(dir)?;
        let mut conn=connect(&dir.join("memory.db"))?;
        let app:i64=conn.query_row("PRAGMA application_id",[],|r|r.get(0))?;
        let version:i64=conn.query_row("PRAGMA user_version",[],|r|r.get(0))?;
        if app!=APP_ID || version!=1 {return Err(Error::UnsupportedSchema);}
        let actual:String=conn.query_row("SELECT owner_scope FROM identity WHERE singleton=1",[],|r|r.get(0))?;
        if actual!=owner {return Err(Error::WrongOwner);}
        configure(&conn)?;
        let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let v:i64=tx.query_row("PRAGMA user_version",[],|r|r.get(0))?;
        if v!=1 {return Err(Error::UnsupportedSchema);}
        tx.execute_batch(POLICY)?;
        tx.pragma_update(None,"user_version",VERSION)?;
        recovery::check_integrity(&tx,owner)?;
        tx.commit()?;
        Ok(())
    }

    pub fn allow_voice(&mut self,s:&str,p:&str,revision:i64,ttl_seconds:i64)->Result<()> {
        if s!="owner" || !matches!(p,"favorite_color"|"favorite_food"|"favorite_music")
            || !(1..=86400).contains(&ttl_seconds) {
            return Err(Error::DisclosureDenied);
        }
        let o=&self.identity.owner_scope;
        let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let fact=current(&tx,o,entity_id(&tx,o,s)?,p)?.ok_or(Error::Missing)?;
        if fact.revision!=revision {return Err(Error::Conflict);}
        if !matches!(fact.value,Value::Text(_)) {return Err(Error::DisclosureDenied);}
        tx.execute("INSERT INTO voice_grant(assertion_id,expires_at) VALUES(?1,unixepoch()+?2)
          ON CONFLICT(assertion_id) DO UPDATE SET expires_at=excluded.expires_at",params![revision,ttl_seconds])?;
        tx.commit()?;
        Ok(())
    }
    pub fn revoke_voice(&mut self,s:&str,p:&str)->Result<usize> {
        key(p)?;
        let o=&self.identity.owner_scope;
        let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let sid=entity_id(&tx,o,s)?;
        let n=tx.execute("DELETE FROM voice_grant WHERE assertion_id IN
            (SELECT id FROM assertion WHERE owner_scope=?1 AND subject_id=?2 AND predicate=?3)",params![o,sid,p])?;
        tx.commit()?;
        Ok(n)
    }
}
