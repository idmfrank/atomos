//! Versioned logical export, not a raw SQL dump. Only fixed prepared
//! statements can restore records. Grants are intentionally not exported.
use super::*;
use rusqlite::types::{Value as SqlValue, ValueRef};
use std::io::{Read,Write};
use std::fs::File;

const MAGIC:&str="ATOMOS-AMG-LOGICAL-1";
const MAX_EXPORT:usize=8*1024*1024;
const MAX_ROWS:usize=20_000;
const MAX_LINE:usize=4096;
const BAD:Error=Error::Invalid("invalid or oversized logical backup");
struct Table {
    tag:&'static str,
    select:&'static str,
    insert:&'static str,
    types:&'static str,
}
const TABLES:[Table;5]=[
    Table{tag:"I",select:"SELECT agent_id,owner_scope,display_name,created_at FROM identity ORDER BY singleton",insert:"INSERT INTO identity(singleton,agent_id,owner_scope,display_name,created_at) VALUES(1,?1,?2,?3,?4)",types:"ttti"},
    Table{tag:"E",select:"SELECT id,owner_scope,entity_key,label,kind FROM entity ORDER BY id",insert:"INSERT INTO entity(id,owner_scope,entity_key,label,kind) VALUES(?1,?2,?3,?4,?5)",types:"itttt"},
    Table{tag:"S",select:"SELECT id,owner_scope,kind,recorded_at FROM source ORDER BY id",insert:"INSERT INTO source(id,owner_scope,kind,recorded_at) VALUES(?1,?2,?3,?4)",types:"itti"},
    Table{tag:"A",select:"SELECT id,owner_scope,subject_id,predicate,value_kind,text_value,int_value,target_id,source_id,recorded_at,superseded_at,supersedes_id FROM assertion ORDER BY id",insert:"INSERT INTO assertion(id,owner_scope,subject_id,predicate,value_kind,text_value,int_value,target_id,source_id,recorded_at,superseded_at,supersedes_id) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",types:"itittTIIiiII"},
    Table{tag:"Q",select:"SELECT name,seq FROM sqlite_sequence WHERE name IN ('entity','source','assertion') ORDER BY name",insert:"INSERT INTO sqlite_sequence(name,seq) VALUES(?1,?2)",types:"ti"},
];
fn hex(bytes:&[u8])->String {
    const H:&[u8]=b"0123456789abcdef";
    let mut s=String::with_capacity(bytes.len()*2);
    for b in bytes {s.push(H[(b>>4) as usize] as char);s.push(H[(b&15) as usize] as char);}
    s
}
fn unhex(s:&str)->Result<String> {
    if s.len()%2!=0||s.len()>2048 {return Err(BAD);}
    fn digit(b:u8)->Option<u8> {match b {b'0'..=b'9'=>Some(b-b'0'),b'a'..=b'f'=>Some(b-b'a'+10),_=>None}}
    let bytes=s.as_bytes().chunks_exact(2).map(|p|Some(digit(p[0])?*16+digit(p[1])?)).collect::<Option<Vec<_>>>().ok_or(BAD)?;
    String::from_utf8(bytes).map_err(|_|BAD)
}
fn encode(v:ValueRef<'_>)->Result<String>{
    match v {
        ValueRef::Null=>Ok("-".into()),
        ValueRef::Integer(i)=>Ok(format!("i{i}")),
        ValueRef::Text(s)=>Ok(format!("t{}",hex(s))),
        _=>Err(BAD),
    }
}
fn decode(s:&str,kind:u8)->Result<SqlValue>{
    if s=="-" && kind.is_ascii_uppercase(){return Ok(SqlValue::Null);}
    match kind.to_ascii_lowercase(){
        b'i'=> {
            let n=s.strip_prefix('i').ok_or(BAD)?.parse::<i64>().map_err(|_|BAD)?;
            if format!("i{n}")!=s{return Err(BAD);}
            Ok(SqlValue::Integer(n))
        }
        b't'=>Ok(SqlValue::Text(unhex(s.strip_prefix('t').ok_or(BAD)?)?)),
        _=>Err(BAD)
    }
}
fn as_text(v:&SqlValue)->Result<&str>{if let SqlValue::Text(s)=v{Ok(s)}else{Err(BAD)}}
fn as_int(v:&SqlValue)->Result<i64>{if let SqlValue::Integer(i)=v{Ok(*i)}else{Err(BAD)}}
fn validate(tag:&str,v:&[SqlValue],owner:&str)->Result<()>{
    match tag{
        "I"=>{
            let id=as_text(&v[0])?;
            if id.len()!=32||!id.bytes().all(|b|b.is_ascii_digit()||matches!(b,b'a'..=b'f')){return Err(BAD);}
            if as_text(&v[1])?!=owner{return Err(Error::WrongOwner);}
            text(as_text(&v[2])?,128)?;
        }
        "E"=>{
            if as_int(&v[0])?<=0||as_text(&v[1])?!=owner{return Err(BAD);}
            key(as_text(&v[2])?)?;text(as_text(&v[3])?,128)?;key(as_text(&v[4])?)?;
        }
        "S"=>{
            if as_int(&v[0])?<=0||as_text(&v[1])?!=owner||as_text(&v[2])?!="owner_local"{return Err(BAD);}
        }
        "A"=>{
            if as_int(&v[0])?<=0||as_text(&v[1])?!=owner{return Err(BAD);}
            key(as_text(&v[3])?)?;
            if let SqlValue::Text(s)=&v[5]{text(s,1024)?;}
            if let SqlValue::Integer(old)=v[11]{if old>=as_int(&v[0])?||old<=0{return Err(BAD);}}
        }
        "Q"=>{
            if !matches!(as_text(&v[0])?,"entity"|"source"|"assertion")||as_int(&v[1])?<0{return Err(BAD);}
        }
        _=>return Err(BAD)
    }
    Ok(())
}
fn append(out:&mut Vec<u8>,line:&str)->Result<()>{
    if line.len()>MAX_LINE||out.len()+line.len()+1>MAX_EXPORT{return Err(BAD);}
    out.extend_from_slice(line.as_bytes());out.push(b'\n');Ok(())
}
/// Check logical and physical consistency before export/after restore/migration.
/// FTS integrity-check verifies index structure; compare content to authoritative rows.
pub(super) fn check_integrity(c:&Connection,owner:&str)->Result<()> {
    let ok:String=c.query_row("PRAGMA integrity_check(1)",[],|r|r.get(0))?;
    if ok!="ok"{return Err(Error::Invalid("database integrity check failed"));}
    let foreign:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check)",[],|r|r.get(0))?;
    if foreign{return Err(Error::Invalid("database reference check failed"));}
    let valid:bool=c.query_row("SELECT
       (SELECT count(*) FROM identity WHERE owner_scope=?1 AND singleton=1)=1
       AND (SELECT count(*) FROM entity WHERE owner_scope=?1 AND entity_key='owner')=1
       AND NOT EXISTS(SELECT 1 FROM assertion a JOIN source s ON a.source_id=s.id WHERE a.recorded_at<>s.recorded_at)
       AND NOT EXISTS(SELECT 1 FROM assertion a LEFT JOIN assertion p ON p.id=a.supersedes_id
         WHERE a.supersedes_id IS NOT NULL AND (p.id IS NULL OR p.id>=a.id OR p.owner_scope<>a.owner_scope OR p.subject_id<>a.subject_id OR p.predicate<>a.predicate OR p.superseded_at IS NULL OR p.superseded_at<>a.recorded_at))
       AND NOT EXISTS(SELECT 1 FROM assertion a WHERE a.superseded_at IS NOT NULL AND (SELECT count(*) FROM assertion b WHERE b.supersedes_id=a.id)<>1)
       AND NOT EXISTS(SELECT 1 FROM voice_grant g JOIN assertion a ON g.assertion_id=a.id JOIN entity e ON e.id=a.subject_id WHERE a.superseded_at IS NOT NULL OR a.value_kind<>'text' OR e.entity_key<>'owner' OR a.predicate NOT IN ('favorite_color','favorite_food','favorite_music'))
       AND NOT EXISTS(SELECT 1 FROM fact_search f LEFT JOIN assertion a ON a.id=f.rowid LEFT JOIN entity t ON t.id=a.target_id
          WHERE a.id IS NULL OR a.superseded_at IS NOT NULL OR f.predicate IS NOT a.predicate OR f.body IS NOT COALESCE(a.text_value,CAST(a.int_value AS TEXT),t.entity_key))
       AND NOT EXISTS(SELECT 1 FROM assertion a LEFT JOIN fact_search f ON f.rowid=a.id WHERE a.superseded_at IS NULL AND f.rowid IS NULL)",[owner],|r|r.get(0))?;
    if !valid{return Err(Error::Invalid("logical memory integrity check failed"));}
    // An FTS control command, not owner supplied SQL.
    c.execute("INSERT INTO fact_search(fact_search) VALUES('integrity-check')",[])?;
    Ok(())
}
fn load(path:&Path,owner:&str)->Result<Vec<(usize,Vec<SqlValue>)>>{
    private_file(path)?;
    let mut bytes=Vec::new();
    File::open(path)?.take((MAX_EXPORT+1) as u64).read_to_end(&mut bytes)?;
    if bytes.len()>MAX_EXPORT||bytes.last()!=Some(&b'\n'){return Err(BAD);}
    let s=std::str::from_utf8(&bytes).map_err(|_|BAD)?;
    let mut lines=s.lines();
    if lines.next()!=Some(MAGIC){return Err(BAD);}
    let mut rows=Vec::new();
    let mut last=0usize;
    let mut ids=BTreeSet::new();
    let mut identity_count=0;
    let mut sequence_names=BTreeSet::new();
    let mut end=false;
    for line in lines {
        if end||line.len()>MAX_LINE{return Err(BAD);}
        let mut fields=line.split('\t');
        let tag=fields.next().ok_or(BAD)?;
        if tag=="END" {
            if fields.next().and_then(|x|x.parse::<usize>().ok())!=Some(rows.len())||fields.next().is_some(){return Err(BAD);}
            end=true;continue;
        }
        let index=TABLES.iter().position(|t|t.tag==tag).ok_or(BAD)?;
        if index<last||rows.len()>=MAX_ROWS{return Err(BAD);}
        if index!=last{ids.clear();last=index;}
        let t=&TABLES[index];
        let values=t.types.bytes().map(|k|decode(fields.next().ok_or(BAD)?,k)).collect::<Result<Vec<_>>>()?;
        if fields.next().is_some(){return Err(BAD);}
        validate(tag,&values,owner)?;
        if tag=="I"{identity_count+=1;if identity_count!=1{return Err(BAD);}}
        if matches!(tag,"E"|"S"|"A")&&!ids.insert(as_int(&values[0])?){return Err(BAD);}
        if tag=="Q"&&!sequence_names.insert(as_text(&values[0])?.to_string()){return Err(BAD);}
        rows.push((index,values));
    }
    if !end||identity_count!=1{return Err(BAD);}
    Ok(rows)
}
impl SqliteMemory {
    pub fn verify(&mut self)->Result<()> {
        let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_integrity(&tx,&self.identity.owner_scope)?;
        tx.commit()?;
        Ok(())
    }
    /// Export into a new private file. Full bounded snapshot, no grants.
    /// The operator must protect the plaintext artifact and its parent directory.
    pub fn export_logical(&mut self,path:&Path)->Result<usize>{
        let parent=path.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        private_dir(parent)?;
        let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_integrity(&tx,&self.identity.owner_scope)?;
        let mut out=Vec::new();
        append(&mut out,MAGIC)?;
        let mut count=0usize;
        for t in &TABLES {
            let mut stmt=tx.prepare(t.select)?;
            let mut rows=stmt.query([])?;
            while let Some(r)=rows.next()?{
                count+=1;if count>MAX_ROWS{return Err(BAD);}
                let mut line=t.tag.to_owned();
                for i in 0..t.types.len(){line.push('\t');line.push_str(&encode(r.get_ref(i)?)?);}
                append(&mut out,&line)?;
            }
        }
        append(&mut out,&format!("END\t{count}"))?;
        tx.commit()?;
        let mut file=OpenOptions::new().write(true).create_new(true).mode(0o600).open(path)?;
        file.write_all(&out)?;file.sync_all()?;File::open(parent)?.sync_all()?;
        Ok(count)
    }
    /// Explicit disaster-recovery operation only. Never merge or overwrite.
    /// Older exports may contain deleted data; caller must reconcile retention.
    pub fn restore_logical(dir:&Path,owner:&str,path:&Path)->Result<Self>{
        key(owner)?;
        let rows=load(path,owner)?;
        DirBuilder::new().mode(0o700).create(dir)?;
        let db=dir.join("memory.db");
        OpenOptions::new().write(true).create_new(true).mode(0o600).open(&db)?;
        let mut conn=connect(&db)?;
        conn.execute_batch("PRAGMA page_size=4096;")?;
        configure(&conn)?;
        let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(SCHEMA)?;
        tx.execute_batch(POLICY)?;
        tx.execute_batch(DELETION_INDEXES)?;
        // The assertion insertion triggers rebuild the active FTS index.
        // Restore AUTOINCREMENT high water marks, not just surviving IDs.
        let mut reset=false;
        for (index,values) in rows {
            if index==4&&!reset {tx.execute("DELETE FROM sqlite_sequence",[])?;reset=true;}
            tx.execute(TABLES[index].insert,rusqlite::params_from_iter(values.iter()))?;
        }
        for table in ["entity","source","assertion"]{
            let max:i64=tx.query_row(&format!("SELECT COALESCE(max(id),0) FROM {table}"),[],|r|r.get(0))?;
            let seq:Option<i64>=tx.query_row("SELECT seq FROM sqlite_sequence WHERE name=?1",[table],|r|r.get(0)).optional()?;
            if seq.unwrap_or(0)<max{return Err(BAD);}
        }
        if !reset{return Err(BAD);}
        check_integrity(&tx,owner)?;
        tx.pragma_update(None,"application_id",APP_ID)?;
        tx.pragma_update(None,"user_version",VERSION)?;
        tx.commit()?;
        drop(conn);
        File::open(dir)?.sync_all()?;
        Self::open(dir,owner)
    }
}
