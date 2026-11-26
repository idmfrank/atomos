#![cfg(unix)]
use atomos_core::{authorization::{AuthorizedReplay,Policy,Envelope,Error,live_inference_enabled},broker::{Request,Event,Usage,Failure,Error as BrokerError},governor::{Priority,Limits}};
use std::{path::PathBuf,fs,os::unix::fs::DirBuilderExt,sync::atomic::{AtomicU64,Ordering}};
static NEXT:AtomicU64=AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture{fn new()->Self{let p=std::env::temp_dir().join(format!("atomos-auth-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));fs::DirBuilder::new().mode(0o700).create(&p).unwrap();Self(p)}}
impl Drop for Fixture{fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
fn policy()->Policy{Policy{account:1,host:2,grant:3,window_start:100,window_end:1000,grant_expiry:900,eligible:true,direct_scope:true,credits_disabled:true,auto_purchase_disabled:true,app_limit_percent:90,limits:Limits::default()}}
fn env(id:u64)->Envelope{Envelope{account:1,host:2,grant:3,request_id:id,now:200,expires_at:230}}
fn req()->Request<'static>{Request{turn:4,task:5,text:"question",priority:Priority::Interactive,max_output:64}}
fn events()->[Event<'static>;3]{[Event::Started{turn:4},Event::Delta{turn:4,text:"synthetic"},Event::Completed{turn:4,usage:Usage{input:35,output:12,cached_input:10,reasoning_output:4}}]}
#[test]
fn live_is_always_disabled_and_missing_evidence_denies(){
    assert!(!live_inference_enabled());
    for p in [Policy{eligible:false,..policy()},Policy{direct_scope:false,..policy()},
        Policy{credits_disabled:false,..policy()},Policy{auto_purchase_disabled:false,..policy()},
        Policy{app_limit_percent:100,..policy()},Policy{app_limit_percent:0,..policy()}]{
        let f=Fixture::new();let mut b=AuthorizedReplay::initialize(&f.0,p).unwrap();
        assert_eq!(b.run(env(1),req(),4,&events()),Err(Error::Evidence));
        assert_eq!(b.charged(),0);assert_eq!(b.dispatches(),0);assert_eq!(b.next_request_id(),1);
    }
}
#[test]
fn account_host_grant_and_expiry_are_bound(){
    let f=Fixture::new();let mut b=AuthorizedReplay::initialize(&f.0,policy()).unwrap();
    for e in [Envelope{account:9,..env(1)},Envelope{host:9,..env(1)},Envelope{grant:9,..env(1)}]{
        assert_eq!(b.run(e,req(),4,&events()),Err(Error::Scope));
    }
    for e in [Envelope{expires_at:200,..env(1)},Envelope{expires_at:261,..env(1)},
        Envelope{now:899,expires_at:901,..env(1)},Envelope{now:900,expires_at:920,..env(1)}]{
        assert_eq!(b.run(e,req(),4,&events()),Err(Error::Expired));
    }
    assert_eq!(b.run(env(1),req(),5,&events()),Err(Error::Broker(BrokerError::StaleTurn)));
    assert_eq!(b.dispatches(),0);assert_eq!(b.next_request_id(),1);
}
#[test]
fn successful_request_and_replay_state_survive_restart(){
    let f=Fixture::new();let mut b=AuthorizedReplay::initialize(&f.0,policy()).unwrap();
    assert_eq!(b.run(env(1),req(),4,&events()).unwrap().text,"synthetic");
    assert_eq!(b.charged(),47);assert_eq!(b.next_request_id(),2);
    assert_eq!(b.run(env(1),req(),4,&events()),Err(Error::Replay));drop(b);
    assert!(matches!(AuthorizedReplay::initialize(&f.0,policy()),Err(Error::Storage)));
    assert!(matches!(AuthorizedReplay::open(&f.0,Policy{app_limit_percent:80,..policy()}),Err(Error::Scope)));
    let mut b=AuthorizedReplay::open(&f.0,policy()).unwrap();
    assert_eq!(b.charged(),47);assert_eq!(b.next_request_id(),2);
    assert_eq!(b.run(Envelope{now:199,..env(2)},req(),4,&events()),Err(Error::ClockRollback));
    assert_eq!(b.run(env(1),req(),4,&events()),Err(Error::Replay));
    assert!(b.run(env(2),req(),4,&events()).is_ok());
    let log=fs::read_to_string(f.0.join("synthetic-authorization-v1/policy.log")).unwrap();
    assert!(!log.contains("question"));assert!(!log.contains("synthetic"));
}
#[test]
fn rate_limit_auth_and_revocation_survive_restart_local_still_works(){
    for kind in [Failure::RateLimited,Failure::Authorization]{
        let f=Fixture::new();let mut b=AuthorizedReplay::initialize(&f.0,policy()).unwrap();
        assert_eq!(b.run(env(1),req(),4,&[Event::Failed{turn:4,kind}]),Err(Error::Broker(BrokerError::Provider(kind))));
        drop(b);let mut b=AuthorizedReplay::open(&f.0,policy()).unwrap();
        assert!(b.suspended());assert_eq!(b.charged(),200);
        assert_eq!(b.run(env(2),req(),4,&events()),Err(Error::Suspended));
        let mut local=req();local.text="help";assert!(!b.run(env(2),local,4,&[]).unwrap().simulated);
    }
    let f=Fixture::new();let mut b=AuthorizedReplay::initialize(&f.0,policy()).unwrap();b.revoke().unwrap();drop(b);
    assert!(AuthorizedReplay::open(&f.0,policy()).unwrap().suspended());
}
#[test]
fn unfinished_intent_fail_closed_and_missing_ledger_not_recreated(){
    use std::io::Write;
    let f=Fixture::new();drop(AuthorizedReplay::initialize(&f.0,policy()).unwrap());
    let path=f.0.join("synthetic-authorization-v1/policy.log");
    let mut file=fs::OpenOptions::new().append(true).open(path).unwrap();
    writeln!(file,"P 1 200 230 4 5 64 8 I").unwrap();file.sync_all().unwrap();drop(file);
    let mut b=AuthorizedReplay::open(&f.0,policy()).unwrap();
    assert!(b.suspended());assert_eq!(b.run(env(2),req(),4,&events()),Err(Error::Suspended));drop(b);
    let journal=f.0.join("synthetic-authorization-v1/budget/synthetic-v1/usage.log");
    fs::remove_file(&journal).unwrap();
    assert!(matches!(AuthorizedReplay::open(&f.0,policy()),Err(Error::Storage)));assert!(!journal.exists());
}
#[test]
fn fixed_window_and_budget_cannot_be_reset_and_unknown_usage_is_retained(){
    let f=Fixture::new();let p=Policy{limits:Limits{total:300,background:100,interactive_reserve:100,per_request:250},..policy()};
    let mut b=AuthorizedReplay::initialize(&f.0,p).unwrap();
    assert_eq!(b.run(env(1),req(),4,&[]),Err(Error::Broker(BrokerError::UnknownUsage)));
    assert_eq!(b.charged(),200);drop(b);
    let mut b=AuthorizedReplay::open(&f.0,p).unwrap();
    assert_eq!(b.run(env(2),req(),4,&events()),Err(Error::Broker(BrokerError::AdmissionDenied)));
    assert_eq!(b.dispatches(),0);assert_eq!(b.charged(),200);assert_eq!(b.next_request_id(),3);drop(b);
    assert!(matches!(AuthorizedReplay::open(&f.0,Policy{window_end:2000,..p}),Err(Error::Scope)));
    assert!(matches!(AuthorizedReplay::open(&f.0,Policy{limits:Limits::default(),..p}),Err(Error::Scope)));
}
#[test]
fn exclusive_writer_and_truncated_log_reject(){
    use std::io::Write;
    let f=Fixture::new();let b=AuthorizedReplay::initialize(&f.0,policy()).unwrap();
    assert!(matches!(AuthorizedReplay::open(&f.0,policy()),Err(Error::Storage)));drop(b);
    let mut file=fs::OpenOptions::new().append(true).open(f.0.join("synthetic-authorization-v1/policy.log")).unwrap();
    write!(file,"P 1").unwrap();file.sync_all().unwrap();
    assert!(matches!(AuthorizedReplay::open(&f.0,policy()),Err(Error::Corrupt)));
}
#[cfg(feature="responses-replay")]
#[test]
fn wire_replay_cannot_read_before_policy_and_matches_fixture_usage(){
    struct Never;
    impl std::io::Read for Never{fn read(&mut self,_:&mut[u8])->std::io::Result<usize>{panic!("read before authorization")}}
    let f=Fixture::new();let mut b=AuthorizedReplay::initialize(&f.0,policy()).unwrap();
    assert_eq!(b.run_sse(Envelope{account:9,..env(1)},req(),4,Never,||false),Err(Error::Scope));
    let wire=include_bytes!("fixtures/responses-text.sse");
    assert!(b.run_sse(env(1),req(),4,wire.as_slice(),||false).is_ok());assert_eq!(b.charged(),47);
}
