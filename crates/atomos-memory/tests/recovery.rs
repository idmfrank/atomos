use atomos_memory::{init,open,open_reader,Error,MemoryStore,MemoryReader,SqliteMemory,Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64,Ordering};
use std::os::unix::fs::DirBuilderExt;
static NEXT:AtomicU64=AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture{
    fn new()->Self{
        let p=std::env::temp_dir().join(format!("atomos-recovery-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
        std::fs::DirBuilder::new().mode(0o700).create(&p).unwrap();
        Self(p)
    }
}
impl Drop for Fixture{fn drop(&mut self){let _=std::fs::remove_dir_all(&self.0);}}
fn v(s:&str)->Value{Value::Text(s.into())}

#[test]
fn logical_roundtrip_preserves_identity_history_and_high_water(){
    let f=Fixture::new();let path=f.0.join("db");let backup=f.0.join("export.amg");
    let mut db=init(&path,"demo","Aster").unwrap();
    db.add_entity("project","Project","project").unwrap();
    db.remember("owner","develops",Value::Entity("project".into())).unwrap();
    let a=db.remember("owner","favorite_color",v("indigo")).unwrap();
    let b=db.correct("owner","favorite_color",a.revision,v("teal")).unwrap();
    let raw=rusqlite::Connection::open(path.join("memory.db")).unwrap();
    raw.execute("INSERT INTO voice_grant(assertion_id,expires_at) VALUES(?1,unixepoch()+3600)",[b.revision]).unwrap();
    drop(raw);
    let deleted=db.remember("project","note",v("forgotten")).unwrap();
    db.forget("project","note").unwrap();
    db.verify().unwrap();
    db.export_logical(&backup).unwrap();
    assert!(db.export_logical(&backup).is_err());
    let dest=f.0.join("restored");
    let mut recovered=SqliteMemory::restore_logical(&dest,"demo",&backup).unwrap();
    assert_eq!(recovered.identity(),db.identity());
    assert_eq!(recovered.history("owner","favorite_color").unwrap(),db.history("owner","favorite_color").unwrap());
    assert_eq!(recovered.search("teal").unwrap(),db.search("teal").unwrap());
    assert!(recovered.search("forgotten").unwrap().facts.is_empty());
    let raw=rusqlite::Connection::open(dest.join("memory.db")).unwrap();
    assert_eq!(raw.query_row("SELECT count(*) FROM voice_grant",[],|r|r.get::<_,i64>(0)).unwrap(),0);
    let new=recovered.remember("project","note",v("new")).unwrap();
    assert!(new.revision>deleted.revision);
    assert!(SqliteMemory::restore_logical(&dest,"demo",&backup).is_err());
    recovered.verify().unwrap();
}
#[test]
fn explicit_v1_migration_preserves_facts_and_rejects_wrong_scope(){
    let f=Fixture::new();let path=f.0.join("db");
    let mut db=init(&path,"demo","Aster").unwrap();
    let fact=db.remember("owner","favorite_color",v("blue")).unwrap();
    let identity=db.identity().clone();drop(db);
    let raw=rusqlite::Connection::open(path.join("memory.db")).unwrap();
    raw.execute_batch("DROP TRIGGER assertion_delete_source; DROP INDEX assertion_source; DROP INDEX assertion_supersedes; DROP TRIGGER voice_grant_retire; DROP TABLE voice_grant; PRAGMA user_version=1;").unwrap();drop(raw);
    assert!(matches!(open(&path,"demo"),Err(Error::UnsupportedSchema)));
    assert!(matches!(SqliteMemory::migrate(&path,"wrong"),Err(Error::WrongOwner)));
    SqliteMemory::migrate(&path,"demo").unwrap();
    let db=open(&path,"demo").unwrap();
    assert_eq!(db.identity(),&identity);assert_eq!(db.recall("owner","favorite_color").unwrap(),Some(fact));
    let raw=rusqlite::Connection::open(path.join("memory.db")).unwrap();
    assert_eq!(raw.query_row("SELECT count(*) FROM voice_grant",[],|r|r.get::<_,i64>(0)).unwrap(),0);
}
#[test]
fn malformed_backup_and_corrupt_index_fail_closed(){
    let f=Fixture::new();let path=f.0.join("db");let backup=f.0.join("export.amg");
    let mut db=init(&path,"demo","Aster").unwrap();
    db.remember("owner","note",v("blue")).unwrap();
    db.export_logical(&backup).unwrap();
    let other=f.0.join("wrong");
    assert!(SqliteMemory::restore_logical(&other,"other",&backup).is_err());assert!(!other.exists());
    let mut bytes=std::fs::read(&backup).unwrap();bytes.pop();std::fs::write(&backup,bytes).unwrap();
    let dest=f.0.join("bad");
    assert!(SqliteMemory::restore_logical(&dest,"demo",&backup).is_err());assert!(!dest.exists());
    let raw=rusqlite::Connection::open(path.join("memory.db")).unwrap();
    raw.execute("DELETE FROM fact_search",[]).unwrap();drop(raw);
    assert!(db.verify().is_err());
    assert!(db.export_logical(&f.0.join("bad.amg")).is_err());
}


#[test]
fn explicit_v2_migration_preserves_voice_grant_and_adds_indexes(){
    let f=Fixture::new();let path=f.0.join("db");
    let mut db=init(&path,"demo","Aster").unwrap();
    let fact=db.remember("owner","favorite_color",v("blue")).unwrap();
    let raw=rusqlite::Connection::open(path.join("memory.db")).unwrap();
    raw.execute("INSERT INTO voice_grant(assertion_id,expires_at) VALUES(?1,unixepoch()+3600)",[fact.revision]).unwrap();
    drop(raw);
    let identity=db.identity().clone();drop(db);
    let raw=rusqlite::Connection::open(path.join("memory.db")).unwrap();
    raw.execute_batch("DROP TRIGGER assertion_delete_source; DROP INDEX assertion_source; DROP INDEX assertion_supersedes; PRAGMA user_version=2;").unwrap();drop(raw);
    assert!(matches!(open(&path,"demo"),Err(Error::UnsupportedSchema)));
    assert!(matches!(SqliteMemory::migrate(&path,"wrong"),Err(Error::WrongOwner)));
    SqliteMemory::migrate(&path,"demo").unwrap();
    let mut db=open(&path,"demo").unwrap();
    assert_eq!(db.identity(),&identity);
    assert_eq!(open_reader(&path,"demo").unwrap().recall("owner","favorite_color").unwrap(),Some(fact.clone()));
    let raw=rusqlite::Connection::open(path.join("memory.db")).unwrap();
    assert_eq!(raw.query_row("SELECT assertion_id FROM voice_grant",[],|r|r.get::<_,i64>(0)).unwrap(),fact.revision);
    let n:i64=raw.query_row("SELECT count(*) FROM sqlite_master WHERE name IN ('assertion_source','assertion_supersedes','assertion_delete_source')",[],|r|r.get(0)).unwrap();
    assert_eq!(n,3);
    db.verify().unwrap();
}
#[test]
fn targeted_forget_preserves_shared_and_unrelated_sources(){
    let f=Fixture::new();let path=f.0.join("db");
    let mut db=init(&path,"demo","Aster").unwrap();
    let a=db.remember("owner","first",v("one")).unwrap();
    let b=db.remember("owner","second",v("two")).unwrap();
    let c=db.remember("owner","third",v("three")).unwrap();
    // Synthetic imported/shared provenance fixture: public writer normally
    // allocates one source per assertion. Preserve consistency of timestamps.
    let raw=rusqlite::Connection::open(path.join("memory.db")).unwrap();
    raw.execute("UPDATE assertion SET source_id=?1,recorded_at=?2 WHERE id=?3",rusqlite::params![a.source_id,a.recorded_at,b.revision]).unwrap();
    raw.execute("DELETE FROM source WHERE id=?1",[b.source_id]).unwrap();
    let source_exists=|id:i64|->bool{
        raw.query_row("SELECT EXISTS(SELECT 1 FROM source WHERE id=?1)",[id],|r|r.get(0)).unwrap()
    };
    assert_eq!(db.forget("owner","first").unwrap(),1);
    assert!(source_exists(a.source_id));assert!(source_exists(c.source_id));
    assert_eq!(db.recall("owner","second").unwrap().unwrap().source_id,a.source_id);
    assert_eq!(db.forget("owner","second").unwrap(),1);
    assert!(!source_exists(a.source_id));assert!(source_exists(c.source_id));
    assert!(db.search("one").unwrap().facts.is_empty());
    assert!(db.search("two").unwrap().facts.is_empty());
    assert_eq!(db.recall("owner","third").unwrap(),Some(c));
    db.verify().unwrap();
}


#[test]
fn streaming_logical_backup_exceeds_old_record_and_byte_caps(){
    let f=Fixture::new();let path=f.0.join("db");let backup=f.0.join("large.amg");
    drop(init(&path,"demo","Aster").unwrap());
    // Bulk synthetic fixture using the actual schema and insert triggers.
    let mut raw=rusqlite::Connection::open(path.join("memory.db")).unwrap();
    raw.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;").unwrap();
    let tx=raw.transaction().unwrap();
    {
        let mut source=tx.prepare("INSERT INTO source(id,owner_scope,kind,recorded_at) VALUES(?1,'demo','owner_local',1700000000)").unwrap();
        let mut fact=tx.prepare("INSERT INTO assertion(id,owner_scope,subject_id,predicate,value_kind,text_value,source_id,recorded_at) VALUES(?1,'demo',1,?2,'text',?3,?1,1700000000)").unwrap();
        let content="synthetic narrative ".repeat(25);
        for i in 1..=10500i64 {
            source.execute([i]).unwrap();
            fact.execute(rusqlite::params![i,format!("note{i}"),&content]).unwrap();
        }
    }
    tx.commit().unwrap();drop(raw);
    let mut db=open(&path,"demo").unwrap();
    let rows=db.export_logical(&backup).unwrap();
    assert!(rows>20_000);
    assert!(std::fs::metadata(&backup).unwrap().len()>8*1024*1024);
    let dest=f.0.join("restored");
    let mut recovered=SqliteMemory::restore_logical(&dest,"demo",&backup).unwrap();
    assert_eq!(recovered.identity(),db.identity());
    for p in ["note1","note5250","note10500"] {
        assert_eq!(recovered.recall("owner",p).unwrap(),db.recall("owner",p).unwrap());
    }
    recovered.verify().unwrap();
}
#[test]
fn oversized_line_is_rejected_before_restore_creation(){
    let f=Fixture::new();
    let path=f.0.join("db");let backup=f.0.join("large.amg");
    let mut db=init(&path,"demo","Aster").unwrap();
    db.export_logical(&backup).unwrap();
    let mut data=std::fs::read(&backup).unwrap();
    data.extend_from_slice(&vec![b'x';5000]);data.push(b'\n');
    std::fs::write(&backup,data).unwrap();
    let dest=f.0.join("bad");
    assert!(SqliteMemory::restore_logical(&dest,"demo",&backup).is_err());
    assert!(!dest.exists());
}

#[test]
fn export_cannot_consume_active_store_budget(){
    let f=Fixture::new();let path=f.0.join("db");
    let mut db=init(&path,"demo","Aster").unwrap();
    let inside=path.join("backup.amg");
    assert!(db.export_logical(&inside).is_err());
    assert!(!inside.exists());
    db.export_logical(&f.0.join("outside.amg")).unwrap();
}
