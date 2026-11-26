//! Versioned logical export, not a raw SQL dump. Only fixed prepared
//! statements can restore records. Grants are intentionally not exported.
use super::*;
use rusqlite::types::{Value as SqlValue, ValueRef};
use std::io::{BufRead,BufReader,BufWriter,Read,Seek,SeekFrom,Write};
use std::fs::File;

const MAGIC:&str="ATOMOS-AMG-LOGICAL-1";
const MAX_EXPORT:usize=256*1024*1024;
const MAX_ROWS:usize=1_000_000;
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
fn append(out:&mut impl Write,total:&mut usize,line:&str)->Result<()>{
    if line.len()>MAX_LINE||*total+line.len()+1>MAX_EXPORT{return Err(BAD);}
    out.write_all(line.as_bytes())?;out.write_all(b"\n")?;
    *total+=line.len()+1;Ok(())
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
// Bounded line reader: a malformed line cannot force unbounded allocation.
// Export uses canonical LF-delimited ASCII fields; hex contains UTF-8 values.
fn next_line(reader:&mut impl BufRead,total:&mut usize,line:&mut Vec<u8>)->Result<bool>{
    line.clear();
    let n=(&mut *reader).take((MAX_LINE+2) as u64).read_until(b'\n',line)?;
    if n==0{return Ok(false);}
    *total+=n;
    if *total>MAX_EXPORT||n>MAX_LINE+1||line.pop()!=Some(b'\n'){return Err(BAD);}
    Ok(true)
}
// Stream validate ordered records; the caller either discards them in a
// preflight pass or applies fixed prepared inserts inside a transaction.
fn parse(reader:&mut impl BufRead,owner:&str,mut consume:impl FnMut(usize,Vec<SqlValue>)->Result<()>)->Result<usize>{
    let mut bytes=0usize;
    let mut line=Vec::with_capacity(MAX_LINE+1);
    if !next_line(reader,&mut bytes,&mut line)?||line!=MAGIC.as_bytes(){return Err(BAD);}
    let mut last=0usize;
    let mut last_id=0i64;
    let mut count=0usize;
    let mut identity_count=0;
    let mut sequence_names=BTreeSet::new();
    let mut end=false;
    while next_line(reader,&mut bytes,&mut line)? {
        if end{return Err(BAD);}
        let line=std::str::from_utf8(&line).map_err(|_|BAD)?;
        let mut fields=line.split('\t');
        let tag=fields.next().ok_or(BAD)?;
        if tag=="END" {
            if fields.next().and_then(|x|x.parse::<usize>().ok())!=Some(count)||fields.next().is_some(){return Err(BAD);}
            end=true;continue;
        }
        let index=TABLES.iter().position(|t|t.tag==tag).ok_or(BAD)?;
        if index<last||count>=MAX_ROWS{return Err(BAD);}
        if index!=last{last_id=0;last=index;}
        let t=&TABLES[index];
        let values=t.types.bytes().map(|k|decode(fields.next().ok_or(BAD)?,k)).collect::<Result<Vec<_>>>()?;
        if fields.next().is_some(){return Err(BAD);}
        validate(tag,&values,owner)?;
        if tag=="I"{identity_count+=1;if identity_count!=1{return Err(BAD);}}
        if matches!(tag,"E"|"S"|"A"){
            let id=as_int(&values[0])?;
            if id<=last_id{return Err(BAD);}
            last_id=id;
        }
        if tag=="Q"&&!sequence_names.insert(as_text(&values[0])?.to_string()){return Err(BAD);}
        consume(index,values)?;
        count+=1;
    }
    if !end||identity_count!=1{return Err(BAD);}
    Ok(count)
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
        let db_path=Path::new(self.conn.path().ok_or(Error::Invalid("missing database path"))?);
        let store=fs::canonicalize(db_path)?.parent().ok_or(Error::Invalid("missing store directory"))?.to_path_buf();
        if fs::canonicalize(parent)?.starts_with(&store){
            return Err(Error::Invalid("backup must be outside the active store directory"));
        }
        let tx=self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        check_integrity(&tx,&self.identity.owner_scope)?;
        let file=OpenOptions::new().write(true).create_new(true).mode(0o600).open(path)?;
        let result=(||->Result<usize>{
            let mut out=BufWriter::with_capacity(64*1024,file);
            let mut bytes=0usize;
            append(&mut out,&mut bytes,MAGIC)?;
            let mut count=0usize;
            for t in &TABLES {
                let mut stmt=tx.prepare(t.select)?;
                let mut rows=stmt.query([])?;
                while let Some(r)=rows.next()?{
                    count+=1;if count>MAX_ROWS{return Err(BAD);}
                    let mut line=t.tag.to_owned();
                    for i in 0..t.types.len(){line.push('\t');line.push_str(&encode(r.get_ref(i)?)?);}
                    append(&mut out,&mut bytes,&line)?;
                }
            }
            append(&mut out,&mut bytes,&format!("END\t{count}"))?;
            out.flush()?;out.get_ref().sync_all()?;
            drop(out);
            tx.commit()?;
            File::open(parent)?.sync_all()?;
            Ok(count)
        })();
        // Never remove a pre-existing file: create_new succeeded above.
        // A crash can still leave an incomplete file; parser rejects it.
        if result.is_err(){let _=fs::remove_file(path);}
        result
    }
    /// Explicit disaster-recovery operation only. Never merge or overwrite.
    /// Older exports may contain deleted data; caller must reconcile retention.
    pub fn restore_logical(dir:&Path,owner:&str,path:&Path)->Result<Self>{
        key(owner)?;
        private_file(path)?;
        let mut input=File::open(path)?;
        if input.metadata()?.len()>MAX_EXPORT as u64{return Err(BAD);}
        // Preflight format/scope before creating the destination. Parse again
        // during insertion using the same open file (trusted local operator).
        let expected=parse(&mut BufReader::new(&mut input),owner,|_,_|Ok(()))?;
        input.seek(SeekFrom::Start(0))?;
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
        let restored=parse(&mut BufReader::new(&mut input),owner,|index,values|{
            if index==4&&!reset {tx.execute("DELETE FROM sqlite_sequence",[])?;reset=true;}
            tx.prepare_cached(TABLES[index].insert)?.execute(rusqlite::params_from_iter(values.iter()))?;
            Ok(())
        })?;
        if restored!=expected{return Err(BAD);}
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
