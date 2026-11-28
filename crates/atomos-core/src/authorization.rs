//! Synthetic account policy, not OAuth or a live authorization grant.
//! Fixed grant/window, no automatic reset, no credentials or network.
//! Linux/private local directory; same-UID tampering is outside this prototype.
use crate::{broker::{self,Broker,Request,Event,Answer,Failure},governor::{Limits,Priority},resolve_local};
use std::{fs::{self,File,OpenOptions},io::{Read,Write},path::{Path,PathBuf}};
use std::os::unix::fs::{MetadataExt,OpenOptionsExt,DirBuilderExt};

const MAX_LOG:u64=1024*1024;
const MAX_WINDOW:u64=31*86400;
const MAX_REQUEST_TTL:u64=60;
const DIR:&str="synthetic-authorization-v1";

#[derive(Clone,Copy)]
pub struct Policy {
    // Opaque synthetic IDs only, not actual provider identifiers.
    pub account:u64,pub host:u64,pub grant:u64,
    pub window_start:u64,pub window_end:u64,pub grant_expiry:u64,
    // Fixture evidence only. These values cannot authorize a live request.
    pub eligible:bool,pub direct_scope:bool,pub credits_disabled:bool,
    pub auto_purchase_disabled:bool,pub app_limit_percent:u8,
    pub limits:Limits,
}
impl Policy {
    pub(crate) fn valid(self)->bool {
        let l=self.limits;
        self.account!=0&&self.host!=0&&self.grant!=0
            &&self.window_start!=0&&self.window_end>self.window_start
            &&self.window_end-self.window_start<=MAX_WINDOW
            &&self.grant_expiry>self.window_start
            &&self.app_limit_percent<=100&&l.total>0&&l.total<=1_000_000_000
            &&l.per_request>0&&l.per_request<=l.total
            &&l.background<=l.total&&l.interactive_reserve<=l.total
    }
    pub(crate) fn included_fixture(self)->bool {
        self.eligible&&self.direct_scope&&self.credits_disabled&&self.auto_purchase_disabled
            &&(1..100).contains(&self.app_limit_percent)
    }
    pub(crate) fn header(self)->String {
        format!("A1 {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
            self.account,self.host,self.grant,self.window_start,self.window_end,self.grant_expiry,
            u8::from(self.eligible),u8::from(self.direct_scope),u8::from(self.credits_disabled),
            u8::from(self.auto_purchase_disabled),self.app_limit_percent,
            self.limits.total,self.limits.background,self.limits.interactive_reserve,self.limits.per_request)
    }
}
/// There is intentionally no live permit or switch in this milestone.
pub const fn live_inference_enabled()->bool{false}

#[derive(Clone,Copy)]
pub struct Envelope {
    pub account:u64,pub host:u64,pub grant:u64,pub request_id:u64,
    /// Injected fixture time, seconds; never a provider reset timestamp.
    pub now:u64,pub expires_at:u64,
}
#[derive(Debug,PartialEq,Eq)]
pub enum Error {
    Invalid, Scope, Evidence, Expired, ClockRollback, Replay, Suspended,
    Storage, Corrupt, Broker(broker::Error),
}
fn storage<T>(r:std::io::Result<T>)->Result<T,Error>{r.map_err(|_|Error::Storage)}
fn private_dir(p:&Path)->Result<(),Error>{
    let m=storage(fs::symlink_metadata(p))?;
    if !m.is_dir()||m.mode()&0o077!=0{return Err(Error::Storage);}Ok(())
}
fn private_file(p:&Path)->Result<(),Error>{
    let m=storage(fs::symlink_metadata(p))?;
    if !m.is_file()||m.mode()&0o077!=0||m.nlink()!=1{return Err(Error::Storage);}Ok(())
}
fn mkdir(p:&Path)->Result<(),Error>{storage(fs::DirBuilder::new().mode(0o700).create(p))}
struct Lock(PathBuf);
impl Drop for Lock{fn drop(&mut self){let _=fs::remove_file(&self.0);}}
fn lock(dir:&Path)->Result<Lock,Error>{
    let p=dir.join("writer.lock");
    let _f=storage(OpenOptions::new().write(true).create_new(true).mode(0o600).open(&p))?;
    Ok(Lock(p))
}
#[derive(Clone,Copy)]
struct Record{ id:u64,now:u64,expiry:u64,turn:u64,task:u64,max_output:u64,bytes:u64,priority:Priority }
impl Record{
    fn line(self)->String{
        format!("P {} {} {} {} {} {} {} {}",self.id,self.now,self.expiry,self.turn,self.task,
            self.max_output,self.bytes,if self.priority==Priority::Interactive{"I"}else{"B"})
    }
    fn valid(self,p:Policy)->bool{
        self.id!=0&&self.id<u64::MAX&&self.turn!=0&&self.task!=0
            &&self.bytes>0&&self.bytes<=1024&&self.max_output>0&&self.max_output<=256
            &&self.now>=p.window_start&&self.now<p.window_end&&self.now<p.grant_expiry
            &&self.expiry>self.now&&self.expiry-self.now<=MAX_REQUEST_TTL
            &&self.expiry<=p.window_end&&self.expiry<=p.grant_expiry
    }
}
struct State{next:u64,last:u64,pending:Option<u64>,halted:bool}
impl State{
    fn new(p:Policy)->Self{Self{next:1,last:p.window_start,pending:None,halted:false}}
    fn replay(&mut self,p:Policy,line:&str)->Result<(),Error>{
        let fields:Vec<_>=line.split(' ').collect();
        let n=|s:&str|s.parse::<u64>().map_err(|_|Error::Corrupt);
        match fields.as_slice(){
            ["P",id,now,expiry,turn,task,max_output,bytes,priority]=>{
                let r=Record{id:n(id)?,now:n(now)?,expiry:n(expiry)?,turn:n(turn)?,task:n(task)?,
                    max_output:n(max_output)?,bytes:n(bytes)?,priority:match *priority{
                        "I"=>Priority::Interactive,"B"=>Priority::Background,_=>return Err(Error::Corrupt)}};
                if !p.included_fixture()||!r.valid(p)||self.halted||self.pending.is_some()||r.id!=self.next
                    ||r.now<self.last||r.line()!=line{return Err(Error::Corrupt);}
                self.next+=1;self.last=r.now;self.pending=Some(r.id);
            }
            ["D",id,halt]=>{
                let id=n(id)?;
                if self.pending!=Some(id)||!matches!(*halt,"0"|"1")||line!=format!("D {id} {halt}"){return Err(Error::Corrupt);}
                self.pending=None;self.halted=*halt=="1";
            }
            ["H"] if self.pending.is_none()&&!self.halted=>self.halted=true,
            _=>return Err(Error::Corrupt),
        }
        Ok(())
    }
}
/// Synchronous synthetic wrapper. Policy and budget journals are separate:
/// durable intent before broker admission; unfinished intent blocks reopen
/// admissions, even if the budget journal was not touched. No retry on crash.
pub struct AuthorizedReplay{
    policy:Policy,state:State,file:File,broker:Broker,_lock:Lock,healthy:bool,
}
impl AuthorizedReplay{
    /// Create once under an existing mode-0700 directory. No overwrite/reset.
    pub fn initialize(root:&Path,p:Policy)->Result<Self,Error>{
        if !p.valid(){return Err(Error::Invalid);}
        private_dir(root)?;
        let dir=root.join(DIR);mkdir(&dir)?;
        let broker_root=dir.join("budget");mkdir(&broker_root)?;
        let broker=Broker::open(&broker_root,p.limits).map_err(Error::Broker)?;
        let _lock=lock(&dir)?;
        let mut file=storage(OpenOptions::new().append(true).read(true).create_new(true).mode(0o600).open(dir.join("policy.log")))?;
        storage(writeln!(file,"{}",p.header()))?;storage(file.sync_all())?;
        storage(File::open(&broker_root).and_then(|f|f.sync_all()))?;
        storage(File::open(&dir).and_then(|f|f.sync_all()))?;storage(File::open(root).and_then(|f|f.sync_all()))?;
        Ok(Self{policy:p,state:State::new(p),file,broker,_lock,healthy:true})
    }
    /// Existing policy must match exactly; absent/mismatched budget is not reset.
    pub fn open(root:&Path,p:Policy)->Result<Self,Error>{
        if !p.valid(){return Err(Error::Invalid);}
        private_dir(root)?;
        let dir=root.join(DIR);private_dir(&dir)?;
        let _lock=lock(&dir)?;
        let path=dir.join("policy.log");private_file(&path)?;
        let mut file=storage(OpenOptions::new().append(true).read(true).open(&path))?;
        let mut text=String::new();storage((&mut file).take(MAX_LOG+1).read_to_string(&mut text))?;
        if text.len() as u64>MAX_LOG||!text.ends_with('\n'){return Err(Error::Corrupt);}
        let mut lines=text.lines();
        if lines.next()!=Some(p.header().as_str()){return Err(Error::Scope);}
        let mut state=State::new(p);
        for line in lines{state.replay(p,line)?;}
        let broker_root=dir.join("budget");private_dir(&broker_root)?;
        // Governor's legacy journal may be mode 0644 under private directories.
        // Require the existing regular, singly-linked file; never recreate it.
        let bd=broker_root.join("synthetic-v1");
        let bm=storage(fs::symlink_metadata(&bd))?;
        let jm=storage(fs::symlink_metadata(bd.join("usage.log")))?;
        if !bm.is_dir()||!jm.is_file()||jm.nlink()!=1{return Err(Error::Storage);}
        let broker=Broker::open(&broker_root,p.limits).map_err(Error::Broker)?;
        Ok(Self{policy:p,state,file,broker,_lock,healthy:true})
    }
    pub fn charged(&self)->u64{self.broker.charged()}
    pub fn dispatches(&self)->u64{self.broker.dispatches()}
    pub fn next_request_id(&self)->u64{self.state.next}
    pub fn suspended(&self)->bool{!self.healthy||self.state.halted||self.state.pending.is_some()}
    fn persist(&mut self,line:&str)->Result<(),Error>{
        if !self.healthy{return Err(Error::Storage);}
        let result=(||{
            if storage(self.file.metadata())?.len().saturating_add(line.len() as u64+1)>MAX_LOG{return Err(Error::Storage);}
            storage(writeln!(self.file,"{line}"))?;storage(self.file.sync_all())?;
            self.state.replay(self.policy,line)
        })();
        if result.is_err(){self.healthy=false;}result
    }
    /// Permanent local revocation in this fixed synthetic grant.
    pub fn revoke(&mut self)->Result<(),Error>{
        if self.suspended(){return Err(Error::Suspended);}self.persist("H")
    }
    fn execute<'a>(&mut self,e:Envelope,r:Request<'a>,active:u64,
        call:impl FnOnce(&mut Broker,Request<'a>,u64)->Result<Answer,broker::Error>)->Result<Answer,Error>{
        if r.turn==0||r.task==0||r.text.trim().is_empty()||r.text.len()>1024
            ||r.text.chars().any(char::is_control)||r.max_output==0||r.max_output>256{return Err(Error::Invalid);}
        if r.turn!=active{return Err(Error::Broker(broker::Error::StaleTurn));}
        // Only the two existing non-private exact local intents bypass policy.
        if resolve_local(r.text).is_some(){return call(&mut self.broker,r,active).map_err(Error::Broker);}
        if self.suspended(){return Err(Error::Suspended);}
        let p=self.policy;
        if (e.account,e.host,e.grant)!=(p.account,p.host,p.grant){return Err(Error::Scope);}
        if !p.included_fixture(){return Err(Error::Evidence);}
        if e.now<self.state.last{return Err(Error::ClockRollback);}
        if e.request_id!=self.state.next{return Err(Error::Replay);}
        let record=Record{id:e.request_id,now:e.now,expiry:e.expires_at,turn:r.turn,task:r.task,
            max_output:r.max_output,bytes:r.text.len() as u64,priority:r.priority};
        if !record.valid(p){return Err(Error::Expired);}
        // No prompt, digest, credentials or provider payload in policy log.
        // Same immutable request travels through this synchronous call.
        self.persist(&record.line())?;
        let result=call(&mut self.broker,r,active);
        let halt=matches!(&result,Err(broker::Error::Provider(Failure::RateLimited|Failure::Authorization)
            |broker::Error::UsageOverrun|broker::Error::LedgerUnavailable|broker::Error::Suspended));
        self.persist(&format!("D {} {}",record.id,u8::from(halt)))?;
        result.map_err(Error::Broker)
    }
    pub fn run(&mut self,e:Envelope,r:Request<'_>,active:u64,events:&[Event<'_>])->Result<Answer,Error>{
        self.execute(e,r,active,|b,r,a|b.run(r,a,events))
    }
    #[cfg(feature="responses-replay")]
    pub fn run_sse(&mut self,e:Envelope,r:Request<'_>,active:u64,input:impl Read,cancelled:impl FnMut()->bool)->Result<Answer,Error>{
        self.execute(e,r,active,|b,r,a|b.run_sse(r,a,input,cancelled))
    }
}
