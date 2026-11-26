use atomos_memory::{compile_context,init,open,Error,MemoryStore,Retrieval,Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64,Ordering};
static NEXT:AtomicU64=AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new()->Self {
        Self(std::env::temp_dir().join(format!("atomos-amg-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed))))
    }
}
impl Drop for Fixture { fn drop(&mut self) { let _=std::fs::remove_dir_all(&self.0); } }
fn v(s:&str)->Value { Value::Text(s.into()) }

#[test]
fn restart_identity_learn_recall_correct_forget() {
    let f=Fixture::new();
    let mut db=init(&f.0,"test","AtomOS").unwrap();
    let identity=db.identity().clone();
    let old=db.remember("owner","favorite_color",v("indigo")).unwrap();
    drop(db);
    let mut db=open(&f.0,"test").unwrap();
    assert_eq!(db.identity(),&identity);
    assert_eq!(db.recall("owner","favorite_color").unwrap(),Some(old.clone()));
    assert!(matches!(db.remember("owner","favorite_color",v("duplicate")),Err(Error::Conflict)));
    let new=db.correct("owner","favorite_color",old.revision,v("teal")).unwrap();
    assert_eq!(new.supersedes,Some(old.revision));
    assert!(matches!(db.correct("owner","favorite_color",old.revision,v("stale")),Err(Error::Conflict)));
    assert!(db.search("indigo").unwrap().facts.is_empty());
    assert_eq!(db.search("teal").unwrap().facts,vec![new.clone()]);
    let history=db.history("owner","favorite_color").unwrap();
    assert_eq!(history.facts.len(),2);
    assert!(history.facts[1].superseded_at.is_some());
    assert_eq!(db.forget("owner","favorite_color").unwrap(),2);
    drop(db);
    let db=open(&f.0,"test").unwrap();
    assert!(db.recall("owner","favorite_color").unwrap().is_none());
    assert!(db.history("owner","favorite_color").unwrap().facts.is_empty());
    assert!(db.search("teal").unwrap().facts.is_empty());
    assert_eq!(db.identity(),&identity);
}

#[test]
fn scope_and_initialization_fail_closed() {
    let f=Fixture::new();
    assert!(open(&f.0,"test").is_err());
    let db=init(&f.0,"test","AtomOS").unwrap();
    assert!(matches!(open(&f.0,"other"),Err(Error::WrongOwner)));
    assert!(init(&f.0,"test","new").is_err());
    assert_eq!(open(&f.0,"test").unwrap().identity(),db.identity());
}

#[test]
fn typed_values_and_invalid_reference_roll_back() {
    let f=Fixture::new(); let mut db=init(&f.0,"test","AtomOS").unwrap();
    let old=db.remember("owner","count",Value::Integer(42)).unwrap();
    assert!(matches!(db.correct("owner","count",old.revision,Value::Entity("missing".into())),Err(Error::Missing)));
    assert_eq!(db.recall("owner","count").unwrap(),Some(old));
    let flag=db.remember("owner","enabled",Value::Boolean(false)).unwrap();
    assert_eq!(db.recall("owner","enabled").unwrap(),Some(flag));
    assert!(db.remember("owner","bad",v("x\n")).is_err());
    assert!(db.remember("owner","bad",v(&"x".repeat(1025))).is_err());
    assert!(db.remember("owner","bad key",v("x")).is_err());
}

#[test]
fn graph_cycles_and_edges_are_bounded() {
    let f=Fixture::new(); let mut db=init(&f.0,"test","AtomOS").unwrap();
    for k in ["a","b","c"] { db.add_entity(k,k,"project").unwrap(); }
    db.remember("owner","develops",Value::Entity("a".into())).unwrap();
    db.remember("a","links",Value::Entity("b".into())).unwrap();
    db.remember("b","links",Value::Entity("a".into())).unwrap();
    db.remember("c","links",Value::Entity("b".into())).unwrap();
    assert_eq!(db.neighbors("owner",1).unwrap().facts.len(),1);
    let r=db.neighbors("owner",3).unwrap();
    assert_eq!(r.facts.len(),4); assert_eq!(r.visited,4); assert!(!r.truncated);
    assert!(db.neighbors("owner",4).is_err());
    for i in 0..34 {
        let k=format!("node{i}");
        db.add_entity(&k,&k,"project").unwrap();
        db.remember("owner",&format!("edge{i}"),Value::Entity(k)).unwrap();
    }
    assert!(db.neighbors("owner",1).unwrap().truncated);
}

#[test]
fn literal_search_and_context_budget() {
    let f=Fixture::new(); let mut db=init(&f.0,"test","AtomOS").unwrap();
    db.remember("owner","note",v("blue OR green")).unwrap();
    db.remember("owner","other",v("green")).unwrap();
    let r=db.search("blue OR green").unwrap();
    assert_eq!(r.facts.len(),1);
    let compiled=compile_context(&r,4096).unwrap();
    assert!(compiled.contains("blue OR green") && compiled.contains("source="));
    assert!(compile_context(&r,8).is_err());
    assert!(compile_context(&Retrieval{truncated:true,..r},4096).is_err());
}

#[test]
fn unsupported_version_is_not_reset() {
    let f=Fixture::new();
    drop(init(&f.0,"test","AtomOS").unwrap());
    let raw=rusqlite::Connection::open(f.0.join("memory.db")).unwrap();
    raw.pragma_update(None,"user_version",99).unwrap();
    drop(raw);
    assert!(matches!(open(&f.0,"test"),Err(Error::UnsupportedSchema)));
    let raw=rusqlite::Connection::open(f.0.join("memory.db")).unwrap();
    assert_eq!(raw.query_row("PRAGMA user_version",[],|r|r.get::<_,i64>(0)).unwrap(),99);
}
