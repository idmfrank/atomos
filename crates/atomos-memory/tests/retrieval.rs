use atomos_memory::{compile_ranked_context,init,open_reader,ContextRequest,MemoryReader,MemoryStore,Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64,Ordering};
static NEXT:AtomicU64=AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new()->Self { Self(std::env::temp_dir().join(format!("atomos-context-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)))) }
}
impl Drop for Fixture { fn drop(&mut self) { let _=std::fs::remove_dir_all(&self.0); } }
fn v(s:&str)->Value { Value::Text(s.into()) }

#[test]
fn scoped_graph_fts_ranking_connectors_and_provenance() {
    let f=Fixture::new(); let mut db=init(&f.0,"demo","Aster").unwrap();
    for k in ["project","milestone","unrelated"] {db.add_entity(k,k,"project").unwrap();}
    let edge=db.remember("project","current",Value::Entity("milestone".into())).unwrap();
    let exact=db.remember("project","status",v("validation passed")).unwrap();
    let related=db.remember("milestone","status",v("validation complete")).unwrap();
    db.remember("unrelated","status",v("validation confidential")).unwrap();
    db.remember("project","irrelevant",v("bananas")).unwrap();
    db.remember("milestone","back",Value::Entity("project".into())).unwrap();
    let reader=open_reader(&f.0,"demo").unwrap();
    let q=ContextRequest::new("project","What is the validation status?");
    let r=reader.retrieve(&q).unwrap();
    assert!(!r.incomplete); assert_eq!(r.omitted,0); assert_eq!(r.visited,2);
    assert_eq!(r.facts.len(),3);
    assert_eq!(r.facts[0].fact.revision,exact.revision);
    assert!(r.facts.iter().any(|x|x.fact.revision==edge.revision&&x.connector));
    assert!(r.facts.iter().any(|x|x.fact.revision==related.revision));
    let p=compile_ranked_context(&r,4096).unwrap();
    assert_eq!(p.bytes,p.data.len()); assert_eq!(p.facts,3);
    assert!(p.data.contains("source=")); assert!(p.data.contains("validation complete"));
    assert!(!p.data.contains("confidential")&&!p.data.contains("bananas"));
    assert_eq!(r,reader.retrieve(&q).unwrap());
}

#[test]
fn required_facts_are_preserved_and_missing_fails_closed() {
    let f=Fixture::new(); let mut db=init(&f.0,"demo","Aster").unwrap();
    db.remember("owner","color",v("blue")).unwrap();
    db.remember("owner","constraint",v("no remote calls")).unwrap();
    let reader=open_reader(&f.0,"demo").unwrap();
    let mut q=ContextRequest::new("owner","blue");
    q.required=vec!["constraint".into()];
    let r=reader.retrieve(&q).unwrap();
    assert!(r.facts[0].required);
    assert!(compile_ranked_context(&r,4096).unwrap().data.contains("no remote calls"));
    assert!(compile_ranked_context(&r,16).is_err());
    q.required.push("missing".into());
    let r=reader.retrieve(&q).unwrap();
    assert_eq!(r.missing_required,vec!["missing".to_string()]);
    assert!(compile_ranked_context(&r,4096).is_err());
    q.required=vec!["constraint".into()]; q.max_facts=1;
    let r=reader.retrieve(&q).unwrap();
    assert_eq!(r.omitted,1); assert!(compile_ranked_context(&r,4096).is_err());
}

#[test]
fn corrections_and_deletion_are_visible_without_reopening_reader() {
    let f=Fixture::new(); let mut db=init(&f.0,"demo","Aster").unwrap();
    let old=db.remember("owner","color",v("indigo")).unwrap();
    let reader=open_reader(&f.0,"demo").unwrap();
    let mut q=ContextRequest::new("owner","indigo");
    assert_eq!(reader.retrieve(&q).unwrap().facts.len(),1);
    db.correct("owner","color",old.revision,v("teal")).unwrap();
    assert!(reader.retrieve(&q).unwrap().facts.is_empty());
    q.question="teal".into();
    assert!(compile_ranked_context(&reader.retrieve(&q).unwrap(),4096).unwrap().data.contains("teal"));
    db.forget("owner","color").unwrap();
    assert!(compile_ranked_context(&reader.retrieve(&q).unwrap(),4096).is_err());
}

#[test]
fn bounds_and_literal_search_cannot_escape_anchor() {
    let f=Fixture::new(); let mut db=init(&f.0,"demo","Aster").unwrap();
    db.add_entity("other","Other","project").unwrap();
    db.remember("owner","note",v("blue")).unwrap();
    db.remember("other","note",v("secret blue")).unwrap();
    let reader=open_reader(&f.0,"demo").unwrap();
    let mut q=ContextRequest::new("owner","blue OR secret*");
    q.depth=0;
    let p=compile_ranked_context(&reader.retrieve(&q).unwrap(),4096).unwrap();
    assert!(!p.data.contains("secret"));
    q.question="one two three four five six seven eight nine".into();
    assert!(reader.retrieve(&q).is_err());
    q.question="blue".into(); q.depth=4;
    assert!(reader.retrieve(&q).is_err());
    q.depth=2; q.anchor="missing".into();
    assert!(reader.retrieve(&q).is_err());
    assert!(open_reader(&f.0,"other").is_err());
}

#[test]
fn graph_work_cap_is_reported_and_context_refused() {
    let f=Fixture::new(); let mut db=init(&f.0,"demo","Aster").unwrap();
    for i in 0..34 {
        let k=format!("node{i}");
        db.add_entity(&k,&k,"project").unwrap();
        db.remember("owner",&format!("edge{i}"),Value::Entity(k)).unwrap();
    }
    db.remember("owner","note",v("blue")).unwrap();
    let reader=open_reader(&f.0,"demo").unwrap();
    let q=ContextRequest::new("owner","blue");
    let r=reader.retrieve(&q).unwrap();
    assert!(r.incomplete); assert!(r.visited<=32);
    assert!(compile_ranked_context(&r,4096).is_err());
}
