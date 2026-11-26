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
    db.allow_voice("owner","favorite_color",b.revision,3600).unwrap();
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
    assert!(matches!(open_reader(&dest,"demo").unwrap().recall_voice("owner","favorite_color"),Err(Error::DisclosureDenied)));
    let new=recovered.remember("project","note",v("new")).unwrap();
    assert!(new.revision>deleted.revision);
    assert!(SqliteMemory::restore_logical(&dest,"demo",&backup).is_err());
    recovered.verify().unwrap();
}
#[test]
fn grant_is_revision_specific_revocable_and_expiring(){
    let f=Fixture::new();let path=f.0.join("db");
    let mut db=init(&path,"demo","Aster").unwrap();
    let a=db.remember("owner","favorite_food",v("pasta")).unwrap();
    let reader=open_reader(&path,"demo").unwrap();
    assert!(matches!(reader.recall_voice("owner","favorite_food"),Err(Error::DisclosureDenied)));
    db.allow_voice("owner","favorite_food",a.revision,3600).unwrap();
    assert_eq!(reader.recall_voice("owner","favorite_food").unwrap(),Some(a.clone()));
    let b=db.correct("owner","favorite_food",a.revision,v("rice")).unwrap();
    assert!(db.allow_voice("owner","favorite_food",a.revision,3600).is_err());
    assert!(matches!(reader.recall_voice("owner","favorite_food"),Err(Error::DisclosureDenied)));
    db.allow_voice("owner","favorite_food",b.revision,3600).unwrap();
    db.revoke_voice("owner","favorite_food").unwrap();
    assert!(reader.recall_voice("owner","favorite_food").is_err());
    db.allow_voice("owner","favorite_food",b.revision,3600).unwrap();
    // Deterministic expiration fixture, no sleep or wall-time race.
    let raw=rusqlite::Connection::open(path.join("memory.db")).unwrap();
    raw.execute("UPDATE voice_grant SET expires_at=1",[]).unwrap();
    assert!(reader.recall_voice("owner","favorite_food").is_err());
    assert!(db.allow_voice("owner","favorite_food",b.revision,86401).is_err());
    db.forget("owner","favorite_food").unwrap();
    assert!(reader.recall_voice("owner","favorite_food").unwrap().is_none());
    let n:i64=raw.query_row("SELECT count(*) FROM voice_grant",[],|r|r.get(0)).unwrap();
    assert_eq!(n,0);
}
#[test]
fn explicit_v1_migration_preserves_facts_and_rejects_wrong_scope(){
    let f=Fixture::new();let path=f.0.join("db");
    let mut db=init(&path,"demo","Aster").unwrap();
    let fact=db.remember("owner","favorite_color",v("blue")).unwrap();
    let identity=db.identity().clone();drop(db);
    let raw=rusqlite::Connection::open(path.join("memory.db")).unwrap();
    raw.execute_batch("DROP TRIGGER voice_grant_retire; DROP TABLE voice_grant; PRAGMA user_version=1;").unwrap();drop(raw);
    assert!(matches!(open(&path,"demo"),Err(Error::UnsupportedSchema)));
    assert!(matches!(SqliteMemory::migrate(&path,"wrong"),Err(Error::WrongOwner)));
    SqliteMemory::migrate(&path,"demo").unwrap();
    let db=open(&path,"demo").unwrap();
    assert_eq!(db.identity(),&identity);assert_eq!(db.recall("owner","favorite_color").unwrap(),Some(fact));
    assert!(open_reader(&path,"demo").unwrap().recall_voice("owner","favorite_color").is_err());
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
