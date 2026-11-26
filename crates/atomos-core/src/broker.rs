//! M2.1 offline-only inference broker. Typed synthetic events, no HTTP,
//! credentials, provider registration, real token estimation or AMG access.
//! No public transport or detachable permit; admission and dispatch are one
//! synchronous operation. The legacy mock CLI remains entirely offline.
use crate::governor::{self,Governor,Limits,Priority};
use crate::resolve_local;
use std::path::Path;

const MAX_INPUT_BYTES:usize=1024;
const MAX_OUTPUT_BYTES:usize=4096;
const MAX_OUTPUT_TOKENS:u64=256;
const MAX_EVENTS:usize=128;
const FIXTURE_FRAMING:u64=128;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Failure { RateLimited, Authorization, Unavailable, Unsupported, Incomplete }
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Usage {
    pub input:u64,
    pub output:u64,
    pub cached_input:u64,
    pub reasoning_output:u64,
}
impl Usage {
    fn total(self)->Option<u64>{
        if self.cached_input>self.input||self.reasoning_output>self.output{return None;}
        // Details are subsets, never added again.
        self.input.checked_add(self.output)
    }
}
#[derive(Clone,Copy,Debug)]
pub enum Event<'a>{
    Started{turn:u64},
    Delta{turn:u64,text:&'a str},
    Completed{turn:u64,usage:Usage},
    Failed{turn:u64,kind:Failure},
    Cancelled{turn:u64},
}
impl Event<'_>{
    fn turn(&self)->u64{
        match *self {
            Self::Started{turn}|Self::Delta{turn,..}|Self::Completed{turn,..}|
            Self::Failed{turn,..}|Self::Cancelled{turn}=>turn,
        }
    }
}
/// Deliberately no memory/context, credentials, tools, model selector or
/// serialized provider body fields. This type is for synthetic replay only.
pub struct Request<'a>{
    pub turn:u64,
    pub task:u64,
    pub text:&'a str,
    pub priority:Priority,
    pub max_output:u64,
}
#[derive(Debug,PartialEq,Eq)]
pub enum Error {
    InvalidRequest, StaleTurn, AdmissionDenied, LedgerUnavailable,
    Suspended, InvalidStream, UnknownUsage, Cancelled,
    Provider(Failure), UsageOverrun,
}
#[derive(Debug,PartialEq,Eq)]
pub struct Answer{
    pub turn:u64,
    pub text:String,
    pub simulated:bool,
    pub usage:Option<Usage>,
}
/// Single-owner, synchronous broker. The dedicated synthetic ledger must
/// never be used as actual provider accounting. No network dependencies.
pub struct Broker {
    governor:Governor,
    dispatches:u64,
    suspended:bool,
}
impl Broker {
    pub fn open(root:&Path,limits:Limits)->Result<Self,Error>{
        let governor=Governor::open(&root.join("synthetic-v1"),limits)
            .map_err(|_|Error::LedgerUnavailable)?;
        Ok(Self{governor,dispatches:0,suspended:false})
    }
    pub fn charged(&self)->u64{self.governor.charged()}
    pub fn dispatches(&self)->u64{self.dispatches}
    /// One synchronous synthetic attempt, no retries. Caller supplies the
    /// currently committed epoch; stale requests never reserve or dispatch.
    /// All text is buffered until terminal validation and durable settlement.
    /// No actual streaming TTS, concurrent cancellation or external egress.
    pub fn run(&mut self,request:Request<'_>,active_turn:u64,events:&[Event<'_>])->Result<Answer,Error>{
        if request.turn==0||request.task==0||request.text.trim().is_empty()
            ||request.text.len()>MAX_INPUT_BYTES||request.text.chars().any(char::is_control)
            ||request.max_output==0||request.max_output>MAX_OUTPUT_TOKENS{
            return Err(Error::InvalidRequest);
        }
        if request.turn!=active_turn{return Err(Error::StaleTurn);}
        if let Some(answer)=resolve_local(request.text){
            return Ok(Answer{turn:request.turn,text:answer.into(),simulated:false,usage:None});
        }
        if self.suspended{return Err(Error::Suspended);}
        // Conservative *fixture* estimate; not a tokenizer or provider quota.
        let input_bound=request.text.len() as u64+FIXTURE_FRAMING;
        let reserved=input_bound+request.max_output;
        // Reservation is durable before the synthetic dispatch. It is bound
        // to this stack frame and never exposed or reused for a second attempt.
        let permit=self.governor.reserve(request.priority,reserved).map_err(|e|match e{
            governor::Error::Denied=>Error::AdmissionDenied,
            _=>Error::LedgerUnavailable,
        })?;
        self.dispatches=self.dispatches.saturating_add(1);
        // All ambiguous outcomes retain the full durable reservation.
        if events.len()>MAX_EVENTS{return Err(Error::InvalidStream);}
        let mut started=false;
        let mut output=String::new();
        for (i,event) in events.iter().enumerate(){
            if event.turn()!=request.turn{return Err(Error::StaleTurn);}
            let last=i+1==events.len();
            match *event{
                Event::Started{..} if !started&&i==0=>started=true,
                Event::Delta{text,..} if started=>{
                    if text.is_empty()||text.chars().any(|c|c.is_control()&&!matches!(c,'\n'|'\t'))
                        ||text.len()>MAX_OUTPUT_BYTES.saturating_sub(output.len()){
                        return Err(Error::InvalidStream);
                    }
                    output.push_str(text);
                }
                Event::Completed{usage,..} if started&&last=>{
                    let actual=usage.total().ok_or(Error::UnknownUsage)?;
                    // Reject impossible fixture arithmetic before appending a
                    // settlement that could make the journal unreplayable.
                    if actual>1_000_000_000{return Err(Error::UnknownUsage);}
                    let overrun=actual>reserved||usage.input>input_bound||usage.output>request.max_output;
                    // A single durable record settles and latches a violation.
                    self.governor.settle_with_halt(permit,actual,overrun).map_err(|_|Error::LedgerUnavailable)?;
                    if overrun{
                        self.suspended=true;
                        return Err(Error::UsageOverrun);
                    }
                    if output.trim().is_empty()||usage.output==0{return Err(Error::InvalidStream);}
                    return Ok(Answer{turn:request.turn,text:output,simulated:true,usage:Some(usage)});
                }
                Event::Failed{kind,..} if last=>{
                    if matches!(kind,Failure::RateLimited|Failure::Authorization){
                        self.suspended=true;
                    }
                    return Err(Error::Provider(kind));
                }
                Event::Cancelled{..} if last=>return Err(Error::Cancelled),
                _=>return Err(Error::InvalidStream),
            }
        }
        Err(Error::UnknownUsage)
    }
}
