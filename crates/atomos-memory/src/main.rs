#![forbid(unsafe_code)]
//! Explicit owner-operated memory administration; no remote disclosure.
use atomos_memory::{compile_context, init, open, Error, Fact, MemoryStore, Result, SqliteMemory, Value};
use std::io::{self, Read};
use std::path::Path;
use std::os::unix::fs::MetadataExt;

fn input_value(kind: &str) -> Result<Value> {
    // Fact values enter via stdin, not process arguments or shell history.
    let mut bytes=Vec::new();
    io::stdin().take(1026).read_to_end(&mut bytes)?;
    if bytes.last()==Some(&b'\n') { bytes.pop(); }
    if bytes.len()>1024 { return Err(Error::Invalid("value too long")); }
    let s=String::from_utf8(bytes).map_err(|_|Error::Invalid("invalid UTF-8"))?;
    match kind {
        "text"=>Ok(Value::Text(s)),
        "integer"=>s.parse().map(Value::Integer).map_err(|_|Error::Invalid("invalid integer")),
        "boolean"=>match s.as_str() { "true"=>Ok(Value::Boolean(true)), "false"=>Ok(Value::Boolean(false)), _=>Err(Error::Invalid("invalid boolean")) },
        "entity"=>Ok(Value::Entity(s)),
        _=>Err(Error::Invalid("type must be text, integer, boolean or entity")),
    }
}
fn show(f:&Fact) {
    println!("revision={} source={} recorded_at={} subject={} predicate={} value={:?} superseded_at={:?}",
        f.revision,f.source_id,f.recorded_at,f.subject,f.predicate,f.value,f.superseded_at);
}
// Quiescent accounting, not a hard filesystem quota. Count all regular
// files in the dedicated store, including rollback journal and sidecars.
fn storage_report(dir:&Path)->Result<()>{
    const BUDGET:u64=256*1024*1024;
    let mut apparent=0u64;let mut allocated=0u64;let mut main=0u64;
    for item in std::fs::read_dir(dir)?{
        let item=item?;
        let m=std::fs::symlink_metadata(item.path())?;
        if !m.is_file()||m.nlink()!=1{return Err(Error::Invalid("unexpected store directory entry"));}
        apparent=apparent.checked_add(m.len()).ok_or(Error::Invalid("storage size overflow"))?;
        allocated=allocated.checked_add(m.blocks().checked_mul(512).ok_or(Error::Invalid("storage size overflow"))?).ok_or(Error::Invalid("storage size overflow"))?;
        if item.file_name()=="memory.db"{main=m.len();}
    }
    println!("main_bytes={main} aggregate_apparent_bytes={apparent} aggregate_allocated_bytes={allocated} target_bytes={BUDGET} over_target={} hard_quota_verified=false",apparent.max(allocated)>BUDGET);
    Ok(())
}
fn run()->Result<()> {
    let args:Vec<String>=std::env::args().skip(1).collect();
    if args.len()<3 { return Err(Error::Invalid("usage: atomos-memory DIR OWNER COMMAND [arguments]")); }
    let dir=Path::new(&args[0]); let owner=&args[1];
    let rest:Vec<&str>=args[2..].iter().map(String::as_str).collect();
    if let ["init",name]=rest.as_slice() {
        let db=init(dir,owner,name)?;
        println!("agent_id={}",db.identity().agent_id);
        return Ok(());
    }
    if rest.as_slice()==["migrate"] {
        SqliteMemory::migrate(dir,owner)?;
        println!("schema migrated");
        return Ok(());
    }
    if let ["restore",path,"--accept-snapshot"] = rest.as_slice() {
        let db=SqliteMemory::restore_logical(dir,owner,Path::new(path))?;
        println!("restored agent_id={}",db.identity().agent_id);
        return Ok(());
    }
    let mut db=open(dir,owner)?;
    match rest.as_slice() {
        ["storage"]=>storage_report(dir)?,
        ["verify"]=>{db.verify()?;println!("integrity ok");},
        ["export",path]=>println!("exported_rows={}",db.export_logical(Path::new(path))?),
        ["identity"]=>println!("agent_id={} name={:?}",db.identity().agent_id,db.identity().display_name),
        ["entity",k,label,kind]=>{ db.add_entity(k,label,kind)?; println!("saved"); },
        ["remember",s,p,k]=>show(&db.remember(s,p,input_value(k)?)?),
        ["correct",s,p,r,k]=>show(&db.correct(s,p,r.parse().map_err(|_|Error::Invalid("invalid revision"))?,input_value(k)?)?),
        ["recall",s,p]=>match db.recall(s,p)? {Some(f)=>show(&f),None=>println!("not found")},
        ["forget",s,p]=>println!("deleted={}",db.forget(s,p)?),
        ["history",s,p]=>{let r=db.history(s,p)?; for f in &r.facts { show(f); } println!("truncated={}",r.truncated);},
        ["search",q]=>{let r=db.search(q)?; for f in &r.facts { show(f); } println!("truncated={}",r.truncated);},
        ["graph",s]=>{let r=db.neighbors(s,2)?; for f in &r.facts { show(f); } println!("truncated={} visited={}",r.truncated,r.visited);},
        ["context",s,p]=>{
            let facts=db.recall(s,p)?.into_iter().collect();
            let r=atomos_memory::Retrieval {facts,truncated:false,visited:0};
            print!("{}",compile_context(&r,4096)?);
        },
        _=>return Err(Error::Invalid("commands: init NAME, migrate, verify, storage, export PATH, restore PATH --accept-snapshot, identity, entity KEY LABEL KIND, remember SUBJECT PREDICATE TYPE, correct SUBJECT PREDICATE REVISION TYPE, recall SUBJECT PREDICATE, forget SUBJECT PREDICATE, history SUBJECT PREDICATE, search PHRASE, graph SUBJECT, context SUBJECT PREDICATE")),
    }
    Ok(())
}
fn main() {
    if let Err(e)=run() { eprintln!("{e}"); std::process::exit(1); }
}
