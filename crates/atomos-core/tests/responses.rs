#![cfg(feature="responses-replay")]
use atomos_core::{broker::{Broker,Error,Event,Failure,Request,Usage},governor::{Limits,Priority},responses::{decode,decode_compat,DecodeError,Terminal}};
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
    let total=": comment\n".repeat(30000);assert_eq!(bad(&total),DecodeError::Limit);
    let s=change("\"delta\":\"Café 🚀\"",&format!("\"delta\":\"{}\"","x".repeat(4097)));assert_eq!(bad(&s),DecodeError::Limit);
    let mut s=wire().split("\n\n").take(4).collect::<Vec<_>>().join("\n\n")+"\n\n";
    for seq in 4..129{s+=&frame(serde_json::json!({"type":"response.output_text.delta","sequence_number":seq,"item_id":"msg_synthetic","output_index":0,"content_index":0,"delta":"x"}));}
    assert_eq!(bad(&s),DecodeError::Limit);
    let depth=format!("data: {}0{}\n\n","[".repeat(150),"]".repeat(150));assert_eq!(bad(&depth),DecodeError::Json);
}


#[test]
fn terminal_snapshot_and_framing_are_independently_checked(){
    let s=wire();let at=s.rfind("\"text\":\"Café 🚀\\nReady.\"").unwrap();
    let mut bad_final=s.clone();
    bad_final.replace_range(at..at+"\"text\":\"Café 🚀\\nReady.\"".len(),"\"text\":\"other\"");
    assert_eq!(bad(&bad_final),DecodeError::Protocol);
    assert_eq!(bad(&format!("event: response.created\n{s}")),DecodeError::Framing);
    let frame_limit=format!("data: {}\ndata: {}\n\n","x".repeat(20_000),"x".repeat(20_000));
    assert_eq!(bad(&frame_limit),DecodeError::Limit);
    let mut invalid=s.clone();invalid.push_str("data: [DONE]\n\n");
    assert_eq!(bad(&invalid),DecodeError::Protocol);
    assert_eq!(bad(&s.replace("\"output_tokens\":12","\"output_tokens\":12.0")),DecodeError::Usage);
}
#[test]
fn response_failure_known_usage_is_settled_and_authorization_suspends(){
    let f=Fixture::new();let mut b=f.open();
    let s=created()+&frame(serde_json::json!({"type":"response.failed","sequence_number":1,"response":{"id":"resp_synthetic","object":"response","status":"failed","error":{"code":"invalid_api_key","message":"synthetic"},"usage":{"input_tokens":35,"output_tokens":0,"total_tokens":35,"input_tokens_details":{"cached_tokens":10},"output_tokens_details":{"reasoning_tokens":0}}}}));
    assert_eq!(b.run_sse(req("question"),1,s.as_bytes(),||false),Err(Error::Provider(Failure::Authorization)));
    assert_eq!(b.charged(),35);
    assert_eq!(b.run_sse(req("question"),1,WIRE,||false),Err(Error::Suspended));
    assert!(b.run_sse(req("help"),1,WIRE,||false).is_ok());
}
#[test]
fn escaped_unicode_and_data_only_events_match_literal_fixture(){
    let escaped=wire().replace("é","\\u00e9").replace("🚀","\\ud83d\\ude80");
    let data_only=escaped.lines().filter(|l|!l.starts_with("event: ")).collect::<Vec<_>>().join("\n")+"\n";
    assert_eq!(decode(data_only.as_bytes(),||false).unwrap().text,TEXT);
}


const COMPAT:&[u8]=include_bytes!("fixtures/responses-compat.sse");
fn compat_wire()->String{std::str::from_utf8(COMPAT).unwrap().to_owned()}
fn compat_bad(s:&str)->DecodeError{decode_compat(s.as_bytes(),||false).0.unwrap_err()}
#[test]
fn compatibility_profile_matches_reference_on_original_wire_and_fragmentation(){
    for size in [1,7,64,4096]{
        let reference=decode(Chunks{data:WIRE,size},||false).unwrap();
        let (candidate,diag)=decode_compat(Chunks{data:WIRE,size},||false);
        assert_eq!(candidate.unwrap(),reference);
        assert_eq!(diag.validated_events,10);
        let (candidate,diag)=decode_compat(Chunks{data:COMPAT,size},||false);
        let candidate=candidate.unwrap();assert_eq!(candidate.text,TEXT);assert_eq!(candidate.terminal,Terminal::Completed(U));
        assert_eq!(diag.validated_events,12);assert_eq!(diag.stage,"terminal");
    }
    // The original reference remains strict and cannot accept this profile.
    assert_eq!(decode(COMPAT,||false),Err(DecodeError::Protocol));
}
#[test]
fn compatibility_queue_is_bound_to_response_and_before_output(){
    let s=compat_wire();
    assert_eq!(compat_bad(&s.replacen("\"type\":\"response.queued\",\"response\":{\"id\":\"resp_synthetic\"","\"type\":\"response.queued\",\"response\":{\"id\":\"resp_wrong\"",1)),DecodeError::Protocol);
    let mut parts:Vec<_>=s.split("\n\n").filter(|p|!p.is_empty()).collect();
    parts.swap(1,2);
    assert_eq!(compat_bad(&(parts.join("\n\n")+"\n\n")),DecodeError::Protocol);
    let bad_queue=s.replacen("\"status\":\"queued\",\"output\":[]","\"status\":\"queued\",\"output\":[{}]",1);
    assert_eq!(compat_bad(&bad_queue),DecodeError::Protocol);
}
#[test]
fn compatibility_annotation_noop_is_explicit_and_bound_to_part(){
    let s=compat_wire();
    for (a,b,err) in [
        ("\"annotation\":null","\"annotation\":{\"type\":\"url_citation\",\"url\":\"https://example.invalid\"}",DecodeError::Unsupported),
        (",\"annotation\":null","",DecodeError::Unsupported),
        ("\"annotation_index\":0","\"annotation_index\":1",DecodeError::Protocol),
        ("\"annotation_index\":0","\"annotation_index\":-1",DecodeError::Protocol),
    ]{assert!(s.contains(a));assert_eq!(compat_bad(&s.replacen(a,b,1)),err);}
    let wrong=s.replacen("\"annotation_index\":0","\"annotation_index\":0,\"annotation_index\":0",1);
    assert_eq!(compat_bad(&wrong),DecodeError::Json);
    let p=s.replace("\"annotations\":[]","\"annotations\":[{\"type\":\"file_citation\"}]");
    assert_eq!(compat_bad(&p),DecodeError::Unsupported);
}
#[test]
fn compatibility_retains_unknown_capability_sequence_and_terminal_denial(){
    let s=compat_wire();
    for kind in ["response.function_call_arguments.delta","response.reasoning_summary_text.delta","response.refusal.delta","response.future_event"]{
        let x=s.replace("response.output_text.annotation.added",kind);
        assert_eq!(compat_bad(&x),DecodeError::Unsupported);
    }
    assert_eq!(compat_bad(&s.replacen("\"sequence_number\":1","\"sequence_number\":2",1)),DecodeError::Protocol);
    assert_eq!(compat_bad(&s.replacen("\"sequence_number\":8","\"sequence_number\":9",1)),DecodeError::Protocol);
    let at=s.rfind("event: response.completed").unwrap();
    assert_eq!(compat_bad(&s[..at]),DecodeError::MissingTerminal);
    assert_eq!(compat_bad(&s.replace("\"total_tokens\":47","\"total_tokens\":48")),DecodeError::Usage);
    assert_eq!(compat_bad(&(s.clone()+&s[at..])),DecodeError::Protocol);
}
#[test]
fn compatibility_failure_never_releases_partial_text_and_event_count_not_sequence_base(){
    let s=compat_wire();
    let at=s.find("event: response.output_text.annotation.added").unwrap();
    let failed=s[..at].to_owned()+&frame(serde_json::json!({"type":"response.failed","sequence_number":8,"response":{"id":"resp_synthetic","object":"response","status":"failed","error":{"code":"subscription_sharing_usage_unavailable"},"usage":null}}));
    let d=decode_compat(failed.as_bytes(),||false).0.unwrap();
    assert_eq!(d.terminal,Terminal::Failed(Failure::Unavailable,None));assert!(d.text.is_empty());
    let mut many=s[..s.find("event: response.output_text.delta").unwrap()].to_owned();
    // Five initial frames, first sequence is 1. Exactly 128 events allowed.
    for seq in 6..=128{many+=&frame(serde_json::json!({"type":"response.output_text.delta","sequence_number":seq,"item_id":"msg_synthetic","output_index":0,"content_index":0,"delta":"x"}));}
    assert_eq!(compat_bad(&many),DecodeError::MissingTerminal);
    many+=&frame(serde_json::json!({"type":"response.output_text.delta","sequence_number":129,"item_id":"msg_synthetic","output_index":0,"content_index":0,"delta":"x"}));
    assert_eq!(compat_bad(&many),DecodeError::Limit);
}
