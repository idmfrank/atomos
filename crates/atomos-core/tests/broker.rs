use atomos_core::broker::{Answer,Broker,Error,Event,Failure,Request,Usage};
use atomos_core::governor::{Limits,Priority};
use std::{fs,path::PathBuf,sync::atomic::{AtomicU64,Ordering}};
static NEXT:AtomicU64=AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture{
    fn new()->Self{
        let path=std::env::temp_dir().join(format!("atomos-broker-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
        fs::create_dir(&path).unwrap();Self(path)
    }
    fn open(&self)->Broker{Broker::open(&self.0,limits()).unwrap()}
}
impl Drop for Fixture{fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
fn limits()->Limits{Limits{total:2000,background:400,interactive_reserve:600,per_request:1500}}
fn req(text:&str)->Request<'_>{Request{turn:1,task:7,text,priority:Priority::Interactive,max_output:64}}
fn usage()->Usage{Usage{input:35,output:12,cached_input:10,reasoning_output:4}}
fn complete()->[Event<'static>;4]{[
    Event::Started{turn:1},Event::Delta{turn:1,text:"Synthetic "},
    Event::Delta{turn:1,text:"answer."},Event::Completed{turn:1,usage:usage()}
]}
#[test]
fn local_and_invalid_requests_never_dispatch(){
    let f=Fixture::new();let mut b=f.open();
    assert_eq!(b.run(req("what is your name?"),1,&[]).unwrap(),Answer{turn:1,text:"I am AtomOS.".into(),simulated:false,usage:None});
    assert_eq!(b.run(req("question"),2,&complete()),Err(Error::StaleTurn));
    assert_eq!(b.run(req(""),1,&complete()),Err(Error::InvalidRequest));
    assert_eq!(b.run(req(&"x".repeat(1025)),1,&complete()),Err(Error::InvalidRequest));
    assert_eq!(b.charged(),0);assert_eq!(b.dispatches(),0);
}
#[test]
fn final_usage_settles_once_without_double_counting(){
    let f=Fixture::new();let mut b=f.open();
    let a=b.run(req("question"),1,&complete()).unwrap();
    assert_eq!(a.text,"Synthetic answer.");assert!(a.simulated);assert_eq!(a.usage,Some(usage()));
    assert_eq!(b.charged(),47);assert_eq!(b.dispatches(),1);
    drop(b);assert_eq!(f.open().charged(),47);
    let log=fs::read_to_string(f.0.join("synthetic-v1/usage.log")).unwrap();
    assert_eq!(log,"R 1 I 200\nS 1 47\n");
    assert!(!log.contains("question"));
}
#[test]
fn denied_admission_never_dispatches(){
    let f=Fixture::new();let mut b=Broker::open(&f.0,Limits{per_request:100,..limits()}).unwrap();
    assert_eq!(b.run(req("question"),1,&complete()),Err(Error::AdmissionDenied));
    assert_eq!(b.dispatches(),0);assert_eq!(b.charged(),0);
}
#[test]
fn cancelled_missing_and_stale_streams_keep_reservations(){
    let f=Fixture::new();let mut b=f.open();
    assert_eq!(b.run(req("question"),1,&[Event::Started{turn:1},Event::Delta{turn:1,text:"partial"},Event::Cancelled{turn:1}]),Err(Error::Cancelled));
    assert_eq!(b.run(req("question"),1,&[Event::Started{turn:1}]),Err(Error::UnknownUsage));
    assert_eq!(b.run(req("question"),1,&[Event::Started{turn:1},Event::Delta{turn:2,text:"stale"}]),Err(Error::StaleTurn));
    assert_eq!(b.charged(),600);assert_eq!(b.dispatches(),3);
    drop(b);assert_eq!(f.open().charged(),600);
}
#[test]
fn terminal_duplicates_and_invalid_usage_fail_closed(){
    let f=Fixture::new();let mut b=f.open();
    let mut events=complete().to_vec();events.push(Event::Completed{turn:1,usage:usage()});
    assert_eq!(b.run(req("question"),1,&events),Err(Error::InvalidStream));
    let mut bad=usage();bad.reasoning_output=13;
    assert_eq!(b.run(req("question"),1,&[Event::Started{turn:1},Event::Completed{turn:1,usage:bad}]),Err(Error::UnknownUsage));
    assert_eq!(b.charged(),400);
}
#[test]
fn output_and_event_caps_never_release_partial_text(){
    let f=Fixture::new();let mut b=f.open();
    let long="x".repeat(4097);
    assert_eq!(b.run(req("question"),1,&[Event::Started{turn:1},Event::Delta{turn:1,text:&long}]),Err(Error::InvalidStream));
    let many=vec![Event::Delta{turn:1,text:"x"};129];
    assert_eq!(b.run(req("question"),1,&many),Err(Error::InvalidStream));
    assert_eq!(b.charged(),400);
}
#[test]
fn quota_and_auth_failures_suspend_without_retry_but_local_survives(){
    for kind in [Failure::RateLimited,Failure::Authorization]{
        let f=Fixture::new();let mut b=f.open();
        assert_eq!(b.run(req("question"),1,&[Event::Failed{turn:1,kind}]),Err(Error::Provider(kind)));
        assert_eq!(b.run(req("question"),1,&complete()),Err(Error::Suspended));
        assert_eq!(b.run(req("help"),1,&[]).unwrap().usage,None);
        assert_eq!(b.dispatches(),1);assert_eq!(b.charged(),200);
    }
}
#[test]
fn total_and_component_overruns_charge_actual_and_halt_after_restart(){
    for actual in [Usage{input:200,output:100,cached_input:0,reasoning_output:0},Usage{input:10,output:65,cached_input:0,reasoning_output:0}]{
        let f=Fixture::new();let mut b=f.open();
        let e=[Event::Started{turn:1},Event::Delta{turn:1,text:"answer"},Event::Completed{turn:1,usage:actual}];
        assert_eq!(b.run(req("question"),1,&e),Err(Error::UsageOverrun));
        assert_eq!(b.charged(),actual.input+actual.output);
        drop(b);let mut b=f.open();
        assert_eq!(b.run(req("question"),1,&complete()),Err(Error::AdmissionDenied));
        assert_eq!(b.dispatches(),0);
    }
}
#[test]
fn background_limit_and_restart_unknown_usage_are_enforced(){
    let f=Fixture::new();let mut b=f.open();
    for _ in 0..2 {
        let mut r=req("question");r.priority=Priority::Background;
        assert_eq!(b.run(r,1,&[]),Err(Error::UnknownUsage));
    }
    drop(b);let mut b=f.open();
    let mut r=req("question");r.priority=Priority::Background;
    assert_eq!(b.run(r,1,&complete()),Err(Error::AdmissionDenied));
    assert_eq!(b.charged(),400);assert_eq!(b.dispatches(),0);
    assert!(b.run(req("question"),1,&complete()).is_ok());
}
