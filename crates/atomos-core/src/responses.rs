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

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Terminal { Completed(Usage), Failed(Failure,Option<Usage>) }
#[derive(Debug,PartialEq,Eq)]
pub struct Decoded {
    // Failed responses never expose partial output.
    pub text:String,
    pub terminal:Terminal,
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
        Some("rate_limit_exceeded"|"insufficient_quota")=>Failure::RateLimited,
        Some("invalid_api_key"|"authentication_error"|"permission_denied")=>Failure::Authorization,
        Some("invalid_request_error"|"unsupported_model"|"model_not_found")=>Failure::Unsupported,
        _=>Failure::Unavailable,
    }
}
#[derive(Clone,Copy,PartialEq,Eq)]
enum Stage{Initial,Created,InProgress,Item,Part,TextDone,PartDone,ItemDone,Terminal}
struct State{
    stage:Stage,
    seq:u64,
    response:String,
    item:String,
    text:String,
    terminal:Option<Terminal>,
}
impl State{
    fn new()->Self{Self{stage:Stage::Initial,seq:0,response:String::new(),item:String::new(),text:String::new(),terminal:None}}
    fn response<'a>(&self,v:&'a Value,status:&str)->Result<&'a Value>{
        let r=object(v,"response")?;
        require(string(r,"object")?=="response"&&string(r,"status")?==status)?;
        let name=id(string(r,"id")?)?;
        require(self.response.is_empty()||self.response==name)?;
        Ok(r)
    }
    fn item(&self,v:&Value,done:bool)->Result<()>{
        if string(v,"type")?!="message"{return Err(DecodeError::Unsupported);}
        require(string(v,"role")?=="assistant"&&string(v,"status")?==if done{"completed"}else{"in_progress"})?;
        let name=id(string(v,"id")?)?;
        require(self.item.is_empty()||self.item==name)?;
        let c=array(v,"content")?;
        if done {require(c.len()==1)?;part(&c[0],&self.text)?;}
        else {require(c.is_empty())?;}
        Ok(())
    }
    fn indices(&self,v:&Value)->Result<()>{
        require(number(v,"output_index")?==0&&number(v,"content_index")?==0&&string(v,"item_id")?==self.item)
    }
    fn event(&mut self,data:&[u8],name:Option<&str>)->Result<()>{
        if self.seq>=MAX_EVENTS{return Err(DecodeError::Limit);}
        require(self.stage!=Stage::Terminal)?;
        let v=json(data)?;
        let kind=string(&v,"type")?;
        require(name.map_or(true,|s|s==kind))?;
        require(number(&v,"sequence_number")?==self.seq)?;
        self.seq+=1;
        match kind{
            "response.created"=>{
                require(self.stage==Stage::Initial)?;
                let r=self.response(&v,"in_progress")?;
                require(array(r,"output")?.is_empty()&&no_value(r,"usage"))?;
                self.response=id(string(r,"id")?)?.to_owned();
                self.stage=Stage::Created;
            }
            "response.in_progress"=>{
                require(self.stage==Stage::Created)?;
                let r=self.response(&v,"in_progress")?;
                require(array(r,"output")?.is_empty()&&no_value(r,"usage"))?;
                self.stage=Stage::InProgress;
            }
            "response.output_item.added"=>{
                require(matches!(self.stage,Stage::Created|Stage::InProgress)&&number(&v,"output_index")?==0)?;
                let item=object(&v,"item")?;
                self.item(item,false)?;
                self.item=id(string(item,"id")?)?.to_owned();
                self.stage=Stage::Item;
            }
            "response.content_part.added"=>{
                require(self.stage==Stage::Item)?;self.indices(&v)?;
                part(object(&v,"part")?,"")?;
                self.stage=Stage::Part;
            }
            "response.output_text.delta"=>{
                require(self.stage==Stage::Part)?;self.indices(&v)?;
                let delta=string(&v,"delta")?;
                if delta.len()>MAX_TEXT.saturating_sub(self.text.len()){return Err(DecodeError::Limit);}
                require(!delta.is_empty()&&text_ok(delta))?;
                self.text.push_str(delta);
            }
            "response.output_text.done"=>{
                require(self.stage==Stage::Part)?;self.indices(&v)?;
                require(string(&v,"text")?==self.text&&!self.text.trim().is_empty())?;
                self.stage=Stage::TextDone;
            }
            "response.content_part.done"=>{
                require(self.stage==Stage::TextDone)?;self.indices(&v)?;
                part(object(&v,"part")?,&self.text)?;
                self.stage=Stage::PartDone;
            }
            "response.output_item.done"=>{
                require(self.stage==Stage::PartDone&&number(&v,"output_index")?==0)?;
                self.item(object(&v,"item")?,true)?;
                self.stage=Stage::ItemDone;
            }
            "response.completed"=>{
                require(self.stage==Stage::ItemDone)?;
                let r=self.response(&v,"completed")?;
                require(no_value(r,"error")&&no_value(r,"incomplete_details"))?;
                let out=array(r,"output")?;
                require(out.len()==1)?;self.item(&out[0],true)?;
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
pub fn decode(mut input:impl Read,mut cancelled:impl FnMut()->bool)->Result<Decoded>{
    let mut frame=Frame::default();let mut state=State::new();
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
    if !matches!(terminal,Terminal::Completed(_)){state.text.clear();}
    Ok(Decoded{text:state.text,terminal})
}
