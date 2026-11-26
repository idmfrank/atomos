#![cfg(feature="responses-replay")]
use atomos_core::{broker::{Broker,Error,Event,Failure,Request,Usage},governor::{Limits,Priority},responses::{decode,DecodeError,Terminal}};
use std::{fs,io::{self,Read},path::PathBuf,sync::atomic::{AtomicU64,Ordering}};
const WIRE:&[u8]=include_bytes!("fixtures/responses-text.sse");
const TEXT:&str="Café 🚀\nReady.";
const U:Usage=Usage{input:35,output:12,cached_input:10,reasoning_output:4};
static NEXT:AtomicU64=AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture{
    fn new()->Self{
        let p=std::env::temp_dir().join(format!("atomos-wire-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
        fs::create_dir(&p).unwrap();Self(p)
    }
    fn open(&self)->Broker{Broker::open(&self.0,Limits::default()).unwrap()}
    fn log(&self)->String{fs::read_to_string(self.0.join("synthetic-v1/usage.log")).unwrap()}
}
impl Drop for Fixture{fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
struct Chunks<'a>{data:&'a [u8],size:usize}
impl Read for Chunks<'_>{
    fn read(&mut self,b:&mut [u8])->io::Result<usize>{
        let n=self.data.len().min(b.len()).min(self.size);
        b[..n].copy_from_slice(&self.data[..n]);self.data=&self.data[n..];Ok(n)
    }
}
fn req(text:&str)->Request<'_>{Request{turn:1,task:7,text,priority:Priority::Interactive,max_output:64}}
fn wire()->String{std::str::from_utf8(WIRE).unwrap().to_owned()}
fn change(a:&str,b:&str)->String{let s=wire();assert!(s.contains(a));s.replacen(a,b,1)}
fn bad(s:&str)->DecodeError{decode(s.as_bytes(),||false).unwrap_err()}
fn frame(v:serde_json::Value)->String{format!("data: {v}\n\n")}
fn created()->String{wire().split("\n\n").next().unwrap().to_owned()+"\n\n"}
#[test]
fn fragmentation_and_line_endings_preserve_unicode_and_usage(){
    for size in [1,2,3,7,31,64,4096]{
        for sep in ["\n","\r\n","\r"]{
            let s=wire().replace('\n',sep);
            let d=decode(Chunks{data:s.as_bytes(),size},||false).unwrap();
            assert_eq!(d.text,TEXT);assert_eq!(d.terminal,Terminal::Completed(U));
        }
    }
    // SSE data fields join with newline, legal JSON whitespace.
    let s=change("data: {\"type\"","data: {\ndata: \"type\"");
    assert_eq!(decode(s.as_bytes(),||false).unwrap().text,TEXT);
    let s=format!(": keepalive\n\n{}: end\n\n",wire());
    assert_eq!(decode(s.as_bytes(),||false).unwrap().text,TEXT);
}
#[test]
fn typed_and_wire_broker_results_and_durable_charges_match(){
    let a=Fixture::new();let b=Fixture::new();
    let mut x=a.open();let mut y=b.open();
    let reference=x.run(req("question"),1,&[Event::Started{turn:1},Event::Delta{turn:1,text:"Café 🚀"},Event::Delta{turn:1,text:"\nReady."},Event::Completed{turn:1,usage:U}]).unwrap();
    let candidate=y.run_sse(req("question"),1,Chunks{data:WIRE,size:1},||false).unwrap();
    assert_eq!(reference,candidate);assert_eq!(x.charged(),47);assert_eq!(y.charged(),47);
    assert_eq!(x.dispatches(),1);assert_eq!(y.dispatches(),1);assert_eq!(a.log(),b.log());
    drop(y);assert_eq!(b.open().charged(),47);
}
#[test]
fn admission_and_local_resolution_precede_any_read(){
    struct Unread;
    impl Read for Unread{fn read(&mut self,_:&mut [u8])->io::Result<usize>{panic!("read before admission")}}
    let f=Fixture::new();let mut b=Broker::open(&f.0,Limits{per_request:100,..Limits::default()}).unwrap();
    assert_eq!(b.run_sse(req("help"),1,Unread,||false).unwrap().usage,None);
    assert_eq!(b.run_sse(req("question"),2,Unread,||false),Err(Error::StaleTurn));
    assert_eq!(b.run_sse(req("question"),1,Unread,||false),Err(Error::AdmissionDenied));
    assert_eq!(b.charged(),0);assert_eq!(b.dispatches(),0);
}
#[test]
fn invalid_utf8_duplicate_keys_and_invalid_json_fail_closed(){
    let mut x=WIRE.to_vec();let i=x.iter().position(|b|*b==0xc3).unwrap();x[i]=0xff;
    assert_eq!(decode(x.as_slice(),||false),Err(DecodeError::Framing));
    assert_eq!(bad(&change("\"sequence_number\":0","\"sequence_number\":0,\"sequence_number\":0")),DecodeError::Json);
    assert_eq!(bad(&change("\"cached_tokens\":10","\"cached_tokens\":10,\"cached_tokens\":0")),DecodeError::Json);
    assert_eq!(bad(&change("\"sequence_number\":0","\"sequence_number\":")),DecodeError::Json);
    assert_eq!(bad(&change("\"delta\":\"Café 🚀\"","\"delta\":\"\\uD800\"")),DecodeError::Json);
}
#[test]
fn event_identity_order_and_final_text_are_verified(){
    for (a,b) in [
        ("event: response.created","event: response.completed"),
        ("\"sequence_number\":1","\"sequence_number\":0"),
        ("\"sequence_number\":4","\"sequence_number\":8"),
        ("\"item_id\":\"msg_synthetic\"","\"item_id\":\"msg_other\""),
        ("\"output_index\":0","\"output_index\":1"),
        ("\"content_index\":0","\"content_index\":1"),
        ("\"text\":\"Café 🚀\\nReady.\"","\"text\":\"forged\""),
        ("\"status\":\"completed\"","\"status\":\"in_progress\""),
    ]{assert_eq!(bad(&change(a,b)),DecodeError::Protocol,"{a}");}
    let s=wire();let at=s.rfind("\"id\":\"resp_synthetic\"").unwrap();
    let mut wrong=s.clone();wrong.replace_range(at..at+"\"id\":\"resp_synthetic\"".len(),"\"id\":\"resp_other\"");
    assert_eq!(bad(&wrong),DecodeError::Protocol);
    let mut duplicate=s.clone();duplicate.push_str(s.split("\n\n").filter(|s|!s.is_empty()).last().unwrap());duplicate.push_str("\n\n");
    assert_eq!(bad(&duplicate),DecodeError::Protocol);
}
#[test]
fn unsupported_content_and_extensions_are_not_executed(){
    assert_eq!(bad(&change("\"type\":\"message\"","\"type\":\"function_call\"")),DecodeError::Unsupported);
    assert_eq!(bad(&change("\"type\":\"output_text\"","\"type\":\"refusal\"")),DecodeError::Unsupported);
    assert_eq!(bad(&change("event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\"","event: response.reasoning_text.delta\ndata: {\"type\":\"response.reasoning_text.delta\"")),DecodeError::Unsupported);
    assert_eq!(bad(&format!("id: resume-token\n{}",wire())),DecodeError::Framing);
}
#[test]
fn usage_validation_and_overrun_settlement(){
    for (a,b) in [("\"total_tokens\":47","\"total_tokens\":48"),("\"cached_tokens\":10","\"cached_tokens\":36"),("\"reasoning_tokens\":4","\"reasoning_tokens\":13"),("\"input_tokens\":35","\"input_tokens\":-1")]{
        assert_eq!(bad(&change(a,b)),DecodeError::Usage);
    }
    let f=Fixture::new();let mut b=f.open();
    let s=wire().replace("\"output_tokens\":12","\"output_tokens\":65").replace("\"total_tokens\":47","\"total_tokens\":100");
    assert_eq!(b.run_sse(req("question"),1,s.as_bytes(),||false),Err(Error::UsageOverrun));
    assert_eq!(b.charged(),100);drop(b);
    let mut b=f.open();
    assert_eq!(b.run_sse(req("question"),1,WIRE,||false),Err(Error::AdmissionDenied));
}
#[test]
fn failures_with_usage_match_typed_replay_and_never_release_text(){
    let terminal=serde_json::json!({"type":"response.incomplete","sequence_number":5,"response":{"id":"resp_synthetic","object":"response","status":"incomplete","usage":{"input_tokens":35,"output_tokens":12,"total_tokens":47,"input_tokens_details":{"cached_tokens":10},"output_tokens_details":{"reasoning_tokens":4}}}});
    let s=wire().split("\n\n").take(5).collect::<Vec<_>>().join("\n\n")+"\n\n"+&frame(terminal);
    let d=decode(s.as_bytes(),||false).unwrap();assert!(d.text.is_empty());assert_eq!(d.terminal,Terminal::Failed(Failure::Incomplete,Some(U)));
    let a=Fixture::new();let b=Fixture::new();let mut x=a.open();let mut y=b.open();
    let expected=x.run(req("question"),1,&[Event::FailedWithUsage{turn:1,kind:Failure::Incomplete,usage:U}]);
    assert_eq!(expected,Err(Error::Provider(Failure::Incomplete)));
    assert_eq!(y.run_sse(req("question"),1,s.as_bytes(),||false),expected);
    assert_eq!(a.log(),b.log());assert_eq!(y.charged(),47);
}
#[test]
fn rate_limit_suspends_unknown_usage_and_local_still_works(){
    let f=Fixture::new();let mut b=f.open();
    let s=created()+&frame(serde_json::json!({"type":"error","sequence_number":1,"code":"rate_limit_exceeded","message":"synthetic"}));
    assert_eq!(b.run_sse(req("question"),1,s.as_bytes(),||false),Err(Error::Provider(Failure::RateLimited)));
    assert_eq!(b.charged(),200);
    assert_eq!(b.run_sse(req("question"),1,WIRE,||false),Err(Error::Suspended));
    assert!(b.run_sse(req("help"),1,WIRE,||false).is_ok());
}
#[test]
fn incomplete_eof_cancel_and_io_retain_reservation(){
    let f=Fixture::new();let mut b=f.open();
    assert_eq!(b.run_sse(req("question"),1,created().as_bytes(),||false),Err(Error::UnknownUsage));
    assert_eq!(b.run_sse(req("question"),1,&WIRE[..WIRE.len()-1],||false),Err(Error::InvalidStream));
    let mut checks=0;
    assert_eq!(b.run_sse(req("question"),1,Chunks{data:WIRE,size:5},||{checks+=1;checks>10}),Err(Error::Cancelled));
    struct Broken;
    impl Read for Broken{fn read(&mut self,_:&mut [u8])->io::Result<usize>{Err(io::Error::from(io::ErrorKind::BrokenPipe))}}
    assert_eq!(b.run_sse(req("question"),1,Broken,||false),Err(Error::UnknownUsage));
    assert_eq!(b.charged(),800);drop(b);assert_eq!(f.open().charged(),800);
}
#[test]
fn byte_event_output_and_depth_limits_fail_closed(){
    let long=format!(": {}\n\n","x".repeat(32769));assert_eq!(bad(&long),DecodeError::Limit);
    let total=": comment\n".repeat(25000);assert_eq!(bad(&total),DecodeError::Limit);
    let s=change("\"delta\":\"Café 🚀\"",&format!("\"delta\":\"{}\"","x".repeat(4097)));assert_eq!(bad(&s),DecodeError::Limit);
    let mut s=wire().split("\n\n").take(4).collect::<Vec<_>>().join("\n\n")+"\n\n";
    for seq in 4..129{s+=&frame(serde_json::json!({"type":"response.output_text.delta","sequence_number":seq,"item_id":"msg_synthetic","output_index":0,"content_index":0,"delta":"x"}));}
    assert_eq!(bad(&s),DecodeError::Limit);
    let depth=format!("data: {}0{}\n\n","[".repeat(150),"]".repeat(150));assert_eq!(bad(&depth),DecodeError::Json);
}
