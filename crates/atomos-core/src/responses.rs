//! Offline, deliberately narrow Responses text-only SSE profile.
//! No HTTP, credentials, logging, tools, reasoning items or live dispatch.
//! One response, one assistant message, one output_text part, explicit terminal.
use crate::broker::{Failure,Usage};
use serde::{Deserialize,Deserializer,de::{self,Visitor,SeqAccess,MapAccess}};
use serde_json::{Value,Map,Number};
use std::{fmt,io::{Read,ErrorKind}};

const MAX_WIRE:usize=256*1024;
const MAX_FRAME:usize=32*1024;
const MAX_EVENTS:u64=128;
const MAX_TEXT:usize=4096;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum DecodeError { Io, Limit, Framing, Json, Protocol, Unsupported, MissingTerminal, Usage, Cancelled }
type Result<T>=std::result::Result<T,DecodeError>;

/// Bounded diagnostics: only static labels and a count, never raw events,
/// IDs, provider metadata or text. Not a second parser or acceptance path.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct DecodeDiagnostics {
    pub stage:&'static str,
    pub event_class:&'static str,
    pub item_class:&'static str,
    pub validated_events:u64,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Terminal { Completed(Usage), Failed(Failure,Option<Usage>) }
#[derive(Debug,PartialEq,Eq)]
pub struct Decoded {
    // Failed responses never expose partial output.
    pub text:String,
    pub terminal:Terminal,
}

/// Separate offline profile result; never passed to the existing broker.
#[derive(Debug,PartialEq,Eq)]
pub enum ExtendedTerminal { Completed(Usage), Refused(Usage), Failed(Failure,Option<Usage>) }
#[derive(Debug,PartialEq,Eq)]
pub struct ExtendedDecoded {
    // Only completed ordinary assistant text is released.
    pub text:String,
    pub terminal:ExtendedTerminal,
}

/// serde_json handles JSON grammar, UTF-8, escapes and recursion limits.
/// Reject duplicate keys at every nesting level instead of accepting a
/// last-wins representation, including in ignored metadata.
struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D:Deserializer<'de>>(d:D)->std::result::Result<Self,D::Error>{
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value=Strict;
            fn expecting(&self,f:&mut fmt::Formatter<'_>)->fmt::Result{f.write_str("JSON without duplicate keys")}
            fn visit_bool<E:de::Error>(self,v:bool)->std::result::Result<Strict,E>{Ok(Strict(Value::Bool(v)))}
            fn visit_i64<E:de::Error>(self,v:i64)->std::result::Result<Strict,E>{Ok(Strict(Value::Number(v.into())))}
            fn visit_u64<E:de::Error>(self,v:u64)->std::result::Result<Strict,E>{Ok(Strict(Value::Number(v.into())))}
            fn visit_f64<E:de::Error>(self,v:f64)->std::result::Result<Strict,E>{
                Number::from_f64(v).map(|n|Strict(Value::Number(n))).ok_or_else(||E::custom("nonfinite number"))
            }
            fn visit_str<E:de::Error>(self,v:&str)->std::result::Result<Strict,E>{Ok(Strict(Value::String(v.into())))}
            fn visit_string<E:de::Error>(self,v:String)->std::result::Result<Strict,E>{Ok(Strict(Value::String(v)))}
            fn visit_unit<E:de::Error>(self)->std::result::Result<Strict,E>{Ok(Strict(Value::Null))}
            fn visit_none<E:de::Error>(self)->std::result::Result<Strict,E>{Ok(Strict(Value::Null))}
            fn visit_seq<A:SeqAccess<'de>>(self,mut a:A)->std::result::Result<Strict,A::Error>{
                let mut v=Vec::new();
                while let Some(Strict(x))=a.next_element::<Strict>()?{v.push(x);}
                Ok(Strict(Value::Array(v)))
            }
            fn visit_map<A:MapAccess<'de>>(self,mut a:A)->std::result::Result<Strict,A::Error>{
                let mut v=Map::new();
                while let Some((k,Strict(x)))=a.next_entry::<String,Strict>()?{
                    if v.insert(k,x).is_some(){return Err(de::Error::custom("duplicate JSON key"));}
                }
                Ok(Strict(Value::Object(v)))
            }
        }
        d.deserialize_any(V)
    }
}
fn json(data:&[u8])->Result<Value>{
    let mut d=serde_json::Deserializer::from_slice(data);
    let Strict(v)=Strict::deserialize(&mut d).map_err(|_|DecodeError::Json)?;
    d.end().map_err(|_|DecodeError::Json)?;
    if !v.is_object(){return Err(DecodeError::Protocol);}
    Ok(v)
}
fn string<'a>(v:&'a Value,key:&str)->Result<&'a str>{
    v.get(key).and_then(Value::as_str).ok_or(DecodeError::Protocol)
}
fn number(v:&Value,key:&str)->Result<u64>{
    v.get(key).and_then(Value::as_u64).ok_or(DecodeError::Protocol)
}
fn object<'a>(v:&'a Value,key:&str)->Result<&'a Value>{
    v.get(key).filter(|x|x.is_object()).ok_or(DecodeError::Protocol)
}
fn array<'a>(v:&'a Value,key:&str)->Result<&'a [Value]>{
    v.get(key).and_then(Value::as_array).map(Vec::as_slice).ok_or(DecodeError::Protocol)
}
fn require(ok:bool)->Result<()>{if ok{Ok(())}else{Err(DecodeError::Protocol)}}
fn id(s:&str)->Result<&str>{
    require(!s.is_empty()&&s.len()<=128&&s.bytes().all(|b|b.is_ascii_alphanumeric()||matches!(b,b'_'|b'-')))?;
    Ok(s)
}
fn text_ok(s:&str)->bool{
    s.len()<=MAX_TEXT&&!s.chars().any(|c|c.is_control()&&!matches!(c,'\n'|'\t'))
}
fn no_value(v:&Value,key:&str)->bool{v.get(key).map_or(true,Value::is_null)}
fn part(v:&Value,expected:&str)->Result<()>{
    if string(v,"type")?!="output_text"{return Err(DecodeError::Unsupported);}
    require(string(v,"text")?==expected)?;
    if let Some(a)=v.get("annotations"){
        if !a.as_array().is_some_and(Vec::is_empty){return Err(DecodeError::Unsupported);}
    }
    Ok(())
}
fn usage(v:&Value)->Result<Usage>{
    let read=||->Result<Usage>{
        let u=Usage{input:number(v,"input_tokens")?,output:number(v,"output_tokens")?,
            cached_input:number(object(v,"input_tokens_details")?,"cached_tokens")?,
            reasoning_output:number(object(v,"output_tokens_details")?,"reasoning_tokens")?};
        require(u.cached_input<=u.input&&u.reasoning_output<=u.output)?;
        require(u.input.checked_add(u.output)==Some(number(v,"total_tokens")?))?;
        if let Some(x)=v["input_tokens_details"].get("cache_write_tokens"){
            require(x.as_u64().is_some_and(|n|n<=u.input))?;
        }
        Ok(u)
    };
    read().map_err(|_|DecodeError::Usage)
}
fn failure(code:Option<&str>)->Failure{
    match code{
        Some("rate_limit_exceeded"|"insufficient_quota"|"subscription_sharing_usage_limit_exceeded")=>Failure::RateLimited,
        Some("invalid_api_key"|"authentication_error"|"permission_denied"|"subscription_sharing_user_not_eligible"|"subscription_sharing_invalid_user"|"chatpass_v2_scope_not_authorized"|"chatpass_v2_invalid_authorization_context")=>Failure::Authorization,
        Some("invalid_request_error"|"unsupported_model"|"model_not_found"|"subscription_sharing_unsupported_capability"|"subscription_sharing_route_not_supported")=>Failure::Unsupported,
        _=>Failure::Unavailable,
    }
}
#[derive(Clone,Copy,PartialEq,Eq)]
enum Stage{Initial,Created,Queued,InProgress,Reasoning,ReasoningDone,Item,Part,TextDone,PartDone,ItemDone,Terminal}
struct State{
    stage:Stage,
    compat:bool,
    extended:bool,
    reasoning_id:Option<String>,
    reasoning_encrypted:Option<String>,
    refusal:bool,
    created_queued:bool,
    null_annotations:u64,
    seq:u64,
    response:String,
    item:String,
    text:String,
    terminal:Option<Terminal>,
    event_class:&'static str,
    item_class:&'static str,
    validated_events:u64,
}
impl State{
    fn new(compat:bool,extended:bool)->Self{Self{stage:Stage::Initial,compat,extended,reasoning_id:None,reasoning_encrypted:None,refusal:false,created_queued:false,null_annotations:0,seq:0,response:String::new(),item:String::new(),text:String::new(),terminal:None,event_class:"none",item_class:"none",validated_events:0}}
    fn diagnostics(&self)->DecodeDiagnostics{
        let stage=match self.stage{Stage::Initial=>"initial",Stage::Created=>"created",Stage::Queued=>"queued",Stage::InProgress=>"in_progress",Stage::Reasoning=>"reasoning",Stage::ReasoningDone=>"reasoning_done",Stage::Item=>"item",Stage::Part=>"part",Stage::TextDone=>"text_done",Stage::PartDone=>"part_done",Stage::ItemDone=>"item_done",Stage::Terminal=>"terminal"};
        DecodeDiagnostics{stage,event_class:self.event_class,item_class:self.item_class,validated_events:self.validated_events}
    }
    fn response<'a>(&self,v:&'a Value,status:&str)->Result<&'a Value>{
        let r=object(v,"response")?;
        require(string(r,"object")?=="response"&&string(r,"status")?==status)?;
        let name=id(string(r,"id")?)?;
        require(self.response.is_empty()||self.response==name)?;
        Ok(r)
    }
    // Narrow opaque reasoning profile: one item, empty summary, no raw
    // reasoning content. Encrypted bytes are only compared in memory, never
    // returned, persisted or replayed into a request.
    fn reasoning<'a>(&self,v:&'a Value,done:bool)->Result<(&'a str,Option<&'a str>)>{
        require(string(v,"type")?=="reasoning")?;
        let name=id(string(v,"id")?)?;
        if !array(v,"summary")?.is_empty(){return Err(DecodeError::Unsupported);}
        if let Some(c)=v.get("content"){
            if !c.is_null()&&!c.as_array().is_some_and(Vec::is_empty){return Err(DecodeError::Unsupported);}
        }
        if !no_value(v,"status"){require(string(v,"status")?==if done{"completed"}else{"in_progress"})?;}
        let encrypted=match v.get("encrypted_content"){
            None|Some(Value::Null)=>None,
            Some(Value::String(s))=>{
                if s.len()>16*1024{return Err(DecodeError::Limit);}
                require(!s.is_empty()&&s.bytes().all(|b|b.is_ascii_graphic()))?;
                Some(s.as_str())
            },
            _=>return Err(DecodeError::Protocol),
        };
        Ok((name,encrypted))
    }
    fn output_index(&self)->u64{u64::from(self.reasoning_id.is_some())}
    fn message_part(&self,v:&Value,expected:&str)->Result<()>{
        if self.refusal{
            require(string(v,"type")?=="refusal"&&string(v,"refusal")?==expected)?;
            require(no_value(v,"text")&&no_value(v,"annotations"))?;
            Ok(())
        }else{part(v,expected)}
    }
    fn item(&self,v:&Value,done:bool)->Result<()>{
        if string(v,"type")?!="message"{return Err(DecodeError::Unsupported);}
        require(string(v,"role")?=="assistant"&&string(v,"status")?==if done{"completed"}else{"in_progress"})?;
        let name=id(string(v,"id")?)?;
        require(self.item.is_empty()||self.item==name)?;
        let c=array(v,"content")?;
        if done {require(c.len()==1)?;self.message_part(&c[0],&self.text)?;}
        else {require(c.is_empty())?;}
        Ok(())
    }
    fn indices(&self,v:&Value)->Result<()>{
        require(number(v,"output_index")?==self.output_index()&&number(v,"content_index")?==0&&string(v,"item_id")?==self.item)
    }
    fn event(&mut self,data:&[u8],name:Option<&str>)->Result<()>{
        self.event_class="unparsed";self.item_class="none";
        if self.validated_events>=MAX_EVENTS{return Err(DecodeError::Limit);}
        require(self.stage!=Stage::Terminal)?;
        let v=json(data)?;
        let kind=string(&v,"type")?;
        self.event_class=match kind{
            "response.created"=>"response.created","response.in_progress"=>"response.in_progress",
            "response.queued"=>"response.queued","response.output_text.annotation.added"=>"response.output_text.annotation.added",
            "response.output_item.added"=>"response.output_item.added","response.output_item.done"=>"response.output_item.done",
            "response.content_part.added"=>"response.content_part.added","response.content_part.done"=>"response.content_part.done",
            "response.output_text.delta"=>"response.output_text.delta","response.output_text.done"=>"response.output_text.done",
            "response.completed"=>"response.completed","response.failed"=>"response.failed","response.incomplete"=>"response.incomplete","error"=>"error",
            "response.reasoning_summary_part.added"|"response.reasoning_summary_part.done"|"response.reasoning_summary_text.delta"|"response.reasoning_summary_text.done"|"response.reasoning_text.delta"|"response.reasoning_text.done"=>"reasoning_event",
            "response.refusal.delta"|"response.refusal.done"=>"refusal_event",_=>"other",
        };
        if matches!(kind,"response.output_item.added"|"response.output_item.done"|"response.content_part.added"|"response.content_part.done"){
            let key=if matches!(kind,"response.output_item.added"|"response.output_item.done"){"item"}else{"part"};
            self.item_class=match v.get(key).and_then(|x|x.get("type")).and_then(Value::as_str){Some("message")=>"message",Some("reasoning")=>"reasoning",Some("output_text")=>"output_text",Some("refusal")=>"refusal",Some("function_call")=>"function_call",Some(_)=>"other",None=>"missing"};
        }
        require(name.map_or(true,|s|s==kind))?;
        let seq=number(&v,"sequence_number")?;
        // Isolated public examples use 1; candidate allows only 0 or 1
        // at the first event and strictly consecutive numbers thereafter.
        if self.compat&&self.validated_events==0&&seq==1{self.seq=1;}
        require(seq==self.seq)?;
        self.seq+=1;
        match kind{
            "response.created"=>{
                require(self.stage==Stage::Initial)?;
                let status=if self.compat&&v.get("response").and_then(|x|x.get("status")).and_then(Value::as_str)==Some("queued"){"queued"}else{"in_progress"};
                let r=self.response(&v,status)?;
                require(array(r,"output")?.is_empty()&&no_value(r,"usage"))?;
                if self.compat{require(no_value(r,"error")&&no_value(r,"incomplete_details"))?;}
                self.created_queued=status=="queued";
                self.response=id(string(r,"id")?)?.to_owned();
                self.stage=Stage::Created;
            }
            "response.queued" if self.compat=>{
                require(self.stage==Stage::Created)?;
                let r=self.response(&v,"queued")?;
                require(array(r,"output")?.is_empty()&&no_value(r,"usage")&&no_value(r,"error")&&no_value(r,"incomplete_details"))?;
                self.stage=Stage::Queued;
            }
            "response.in_progress"=>{
                require(self.stage==Stage::Created||(self.compat&&self.stage==Stage::Queued))?;
                let r=self.response(&v,"in_progress")?;
                require(array(r,"output")?.is_empty()&&no_value(r,"usage"))?;
                if self.compat{require(no_value(r,"error")&&no_value(r,"incomplete_details"))?;}
                self.stage=Stage::InProgress;
            }
            "response.output_item.added"=>{
                require(matches!(self.stage,Stage::Created|Stage::InProgress|Stage::ReasoningDone)&&(!self.created_queued||matches!(self.stage,Stage::InProgress|Stage::ReasoningDone))&&number(&v,"output_index")?==self.output_index())?;
                let item=object(&v,"item")?;
                if self.extended&&string(item,"type")?=="reasoning"{
                    require(self.reasoning_id.is_none()&&self.stage!=Stage::ReasoningDone)?;
                    let (name,_)=self.reasoning(item,false)?;
                    self.reasoning_id=Some(name.to_owned());
                    self.stage=Stage::Reasoning;
                }else{
                    self.item(item,false)?;
                    let name=id(string(item,"id")?)?;
                    require(self.reasoning_id.as_deref()!=Some(name))?;
                    self.item=name.to_owned();
                    self.stage=Stage::Item;
                }
            }
            "response.content_part.added"=>{
                require(self.stage==Stage::Item)?;self.indices(&v)?;
                let p=object(&v,"part")?;
                if self.extended&&string(p,"type")?=="refusal"{self.refusal=true;}
                self.message_part(p,"")?;
                self.stage=Stage::Part;
            }
            "response.output_text.delta"|"response.refusal.delta" if kind=="response.output_text.delta"||self.extended=>{
                require(self.stage==Stage::Part&&(kind=="response.refusal.delta")==self.refusal)?;self.indices(&v)?;
                let delta=string(&v,"delta")?;
                if delta.len()>MAX_TEXT.saturating_sub(self.text.len()){return Err(DecodeError::Limit);}
                require(!delta.is_empty()&&text_ok(delta))?;
                self.text.push_str(delta);
            }
            "response.output_text.annotation.added" if self.compat=>{
                // Only documented null annotation payloads are no-ops. The
                // terminal text profile still requires no actual annotations.
                require(self.stage==Stage::Part&&!self.refusal)?;self.indices(&v)?;
                if !v.get("annotation").is_some_and(Value::is_null){return Err(DecodeError::Unsupported);}
                require(number(&v,"annotation_index")?==self.null_annotations)?;
                self.null_annotations+=1;
            }
            "response.output_text.done"|"response.refusal.done" if kind=="response.output_text.done"||self.extended=>{
                require(self.stage==Stage::Part&&(kind=="response.refusal.done")==self.refusal)?;self.indices(&v)?;
                let key=if self.refusal{"refusal"}else{"text"};
                require(string(&v,key)?==self.text&&!self.text.trim().is_empty())?;
                self.stage=Stage::TextDone;
            }
            "response.content_part.done"=>{
                require(self.stage==Stage::TextDone)?;self.indices(&v)?;
                self.message_part(object(&v,"part")?,&self.text)?;
                self.stage=Stage::PartDone;
            }
            "response.output_item.done" if self.extended&&self.stage==Stage::Reasoning=>{
                require(number(&v,"output_index")?==0)?;
                let (name,encrypted)=self.reasoning(object(&v,"item")?,true)?;
                require(self.reasoning_id.as_deref()==Some(name))?;
                self.reasoning_encrypted=encrypted.map(str::to_owned);
                self.stage=Stage::ReasoningDone;
            }
            "response.output_item.done"=>{
                require(self.stage==Stage::PartDone&&number(&v,"output_index")?==self.output_index())?;
                self.item(object(&v,"item")?,true)?;
                self.stage=Stage::ItemDone;
            }
            "response.completed"=>{
                require(self.stage==Stage::ItemDone)?;
                let r=self.response(&v,"completed")?;
                require(no_value(r,"error")&&no_value(r,"incomplete_details"))?;
                let out=array(r,"output")?;
                if let Some(name)=self.reasoning_id.as_deref(){
                    require(out.len()==2)?;
                    let (final_id,encrypted)=self.reasoning(&out[0],true)?;
                    require(name==final_id&&self.reasoning_encrypted.as_deref()==encrypted)?;
                    self.item(&out[1],true)?;
                }else{require(out.len()==1)?;self.item(&out[0],true)?;}
                let u=usage(r.get("usage").ok_or(DecodeError::Usage)?)?;
                require(u.output>0)?;
                self.terminal=Some(Terminal::Completed(u));self.stage=Stage::Terminal;
            }
            "response.failed"|"response.incomplete"=>{
                require(self.stage!=Stage::Initial)?;
                let incomplete=kind=="response.incomplete";
                let r=self.response(&v,if incomplete{"incomplete"}else{"failed"})?;
                let f=if incomplete{Failure::Incomplete}else{failure(r.get("error").and_then(|e|e.get("code")).and_then(Value::as_str))};
                let u=if no_value(r,"usage"){None}else{Some(usage(&r["usage"])?)};
                self.terminal=Some(Terminal::Failed(f,u));self.stage=Stage::Terminal;
            }
            "error"=>{
                let code=v.get("code").and_then(Value::as_str);
                self.terminal=Some(Terminal::Failed(failure(code),None));self.stage=Stage::Terminal;
            }
            _=>return Err(DecodeError::Unsupported),
        }
        self.validated_events+=1;
        Ok(())
    }
}
#[derive(Default)]
struct Frame{line:Vec<u8>,data:Vec<u8>,name:Option<String>,has_data:bool,skip_lf:bool}
impl Frame{
    fn line(&mut self,state:&mut State)->Result<()>{
        let mut line=std::mem::take(&mut self.line);
        let result=(||->Result<()>{
            let s=std::str::from_utf8(&line).map_err(|_|DecodeError::Framing)?;
            if s.is_empty(){
                if self.has_data{
                    state.event(&self.data,self.name.as_deref())?;
                    self.data.clear();self.has_data=false;self.name=None;
                }else if self.name.is_some(){return Err(DecodeError::Framing);}
                return Ok(());
            }
            if s.starts_with(':'){return Ok(());}
            let (field,value)=s.split_once(':').unwrap_or((s,""));
            let value=value.strip_prefix(' ').unwrap_or(value);
            match field{
                "event"=>{
                    if self.name.is_some()||value.is_empty()||value.len()>128{return Err(DecodeError::Framing);}
                    self.name=Some(value.to_owned());
                }
                "data"=>{
                    if value.len()+(if self.has_data{1}else{0})>MAX_FRAME.saturating_sub(self.data.len()){
                        return Err(DecodeError::Limit);
                    }
                    if self.has_data{self.data.push(b'\n');}
                    self.data.extend_from_slice(value.as_bytes());self.has_data=true;
                }
                // No resumable event IDs, retries, or other SSE extensions.
                _=>return Err(DecodeError::Framing),
            }
            Ok(())
        })();
        line.clear();self.line=line;result
    }
    fn byte(&mut self,b:u8,state:&mut State)->Result<()>{
        if self.skip_lf{self.skip_lf=false;if b==b'\n'{return Ok(());}}
        if b==b'\r'||b==b'\n'{
            self.skip_lf=b==b'\r';
            return self.line(state);
        }
        if self.line.len()>=MAX_FRAME{return Err(DecodeError::Limit);}
        self.line.push(b);Ok(())
    }
}
/// Decode one complete finite replay. No partial text is released. The caller
/// supplies a cooperative cancellation check; a blocking Read is not timed.
/// Limit includes comments, framing and ignored metadata. EOF is required
/// after the terminal frame so duplicate/trailing events cannot be hidden.
pub fn decode(input:impl Read,cancelled:impl FnMut()->bool)->Result<Decoded>{
    decode_diagnostic(input,cancelled).0
}
/// Same parser, with fixed-label diagnostic snapshot on success or failure.
pub fn decode_diagnostic(input:impl Read,cancelled:impl FnMut()->bool)->(Result<Decoded>,DecodeDiagnostics){
    let (result,diag,_)=decode_profile(input,cancelled,false,false);
    (result,diag)
}
/// Offline candidate only. No existing broker, live CLI or qualification
/// command selects this profile. Preserve strict decoding as the reference.
pub fn decode_compat(input:impl Read,cancelled:impl FnMut()->bool)->(Result<Decoded>,DecodeDiagnostics){
    let (result,diag,_)=decode_profile(input,cancelled,true,false);
    (result,diag)
}
/// Separate offline no-tools profile, not selected by any live command.
pub fn decode_extended(input:impl Read,cancelled:impl FnMut()->bool)->(Result<ExtendedDecoded>,DecodeDiagnostics){
    let (result,diag,refusal)=decode_profile(input,cancelled,true,true);
    let result=result.map(|d|{
        let terminal=match d.terminal{
            Terminal::Completed(u) if refusal=>ExtendedTerminal::Refused(u),
            Terminal::Completed(u)=>ExtendedTerminal::Completed(u),
            Terminal::Failed(f,u)=>ExtendedTerminal::Failed(f,u),
        };
        ExtendedDecoded{text:d.text,terminal}
    });
    (result,diag)
}
fn decode_profile(mut input:impl Read,mut cancelled:impl FnMut()->bool,compat:bool,extended:bool)->(Result<Decoded>,DecodeDiagnostics,bool){
    let mut frame=Frame::default();let mut state=State::new(compat,extended);
    let result=(||->Result<Decoded>{
        let mut total=0usize;let mut buf=[0u8;4096];
        loop{
            if cancelled(){return Err(DecodeError::Cancelled);}
            let n=match input.read(&mut buf){
                Ok(n) if n<=buf.len()=>n,
                Ok(_)=>return Err(DecodeError::Io),
                Err(e) if e.kind()==ErrorKind::Interrupted=>return Err(DecodeError::Cancelled),
                Err(_)=>return Err(DecodeError::Io),
            };
            if n==0{break;}
            total+=n;if total>MAX_WIRE{return Err(DecodeError::Limit);}
            for b in &buf[..n]{frame.byte(*b,&mut state)?;}
        }
        if cancelled(){return Err(DecodeError::Cancelled);}
        if !frame.line.is_empty()||frame.has_data||frame.name.is_some(){return Err(DecodeError::Framing);}
        let terminal=state.terminal.ok_or(DecodeError::MissingTerminal)?;
        if state.refusal||!matches!(terminal,Terminal::Completed(_)){state.text.clear();}
        Ok(Decoded{text:std::mem::take(&mut state.text),terminal})
    })();
    (result,state.diagnostics(),state.refusal)
}
