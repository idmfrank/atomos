// Explicit one-attempt qualification only; never a general inference API.
import https from 'node:https';
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {performance} from 'node:perf_hooks';
import {AuthError,require,catalogAllowed,RESOURCE} from './protocol.mjs';
import {strictJson} from './strict-json.mjs';

export const MODEL='gpt-6.2-sol';
export const PROMPT='Say exactly: Hello, world!';
export const ACK='--confirm-personal-app-below-100-and-uncapped-single-request';
const ENDPOINT=RESOURCE+'/responses';
const DEADLINE=30000,MAX=256*1024,RESERVED=1024;
export function body(){return JSON.stringify({model:MODEL,input:[{role:'user',content:PROMPT}],store:false,stream:true});}
function hash(s){return createHash('sha256').update(s).digest('hex');}
export function binding(p){return hash(JSON.stringify([p.host,p.client_id,p.subject]));}
export function intent(p,now=Date.now()){
  return {version:1,state:'pending',binding:binding(p),request_sha256:hash(body()),model:MODEL,created_ms:now,reserved_planning_tokens:RESERVED};
}
function integer(v,max=1000000000){return Number.isSafeInteger(v)&&v>=0&&v<=max;}
export function resultRecord(pending,decoded,metrics){
  const u=decoded?.usage;
  const valid=u&&integer(u.input)&&integer(u.output)&&integer(u.cached_input)&&integer(u.reasoning_output)&&u.cached_input<=u.input&&u.reasoning_output<=u.output&&u.input+u.output<=1000000000;
  const complete=decoded?.terminal==='completed'&&valid&&u.output>0&&typeof decoded.expected_text==='boolean'&&integer(decoded.text_bytes,4096);
  const failed=decoded?.terminal==='failed'&&valid&&['RateLimited','Authorization','Unavailable','Unsupported','Incomplete'].includes(decoded.category);
  const known=complete||failed;
  const actual=known?u.input+u.output:null;
  return {...pending,state:known?'settled':'unknown',outcome:complete?(actual>RESERVED?'overrun':decoded.expected_text?'qualified':'unexpected_text'):failed?'provider_failed':'unvalidated',reported_tokens:known?{input:u.input,output:u.output,cached_input:u.cached_input,reasoning_output:u.reasoning_output,total:actual}:null,charged_planning_units:known?actual:RESERVED,unknown_usage:!known,expected_text:complete?decoded.expected_text:null,category:failed?decoded.category:null,...metrics};
}
// Exactly one HTTP request, no redirect handling, agent reuse, retry,
// SDK or API-key fallback. Absolute wall deadline includes connection/body.
export function sendOnce(access){
  return new Promise((resolve,reject)=>{
    const payload=body(),start=performance.now();
    let timer,finished=false,req,res,first=null,headers=null,total=0,chunks=[];
    const finish=(err,value)=>{
      if(finished)return;finished=true;clearTimeout(timer);
      if(err){res?.destroy();req?.destroy();reject(err);}else resolve(value);
    };
    try{
      require(process.env.NODE_TLS_REJECT_UNAUTHORIZED!=='0','unsafe_tls');
      req=https.request(ENDPOINT,{method:'POST',agent:false,rejectUnauthorized:true,headers:{authorization:'Bearer '+access,'content-type':'application/json','accept':'text/event-stream','accept-encoding':'identity','content-length':Buffer.byteLength(payload)}},r=>{
        res=r;headers=performance.now()-start;
        if(r.statusCode!==200){finish(new AuthError('probe_http_'+r.statusCode));return;}
        if(!/^text\/event-stream(?:\s*;|$)/i.test(r.headers['content-type']??'')||r.headers['content-encoding']&&r.headers['content-encoding']!=='identity'){finish(new AuthError('probe_content_type'));return;}
        r.on('data',b=>{
          if(finished)return;
          if(first===null)first=performance.now()-start;
          total+=b.length;
          if(total>MAX){finish(new AuthError('probe_wire_limit'));return;}
          chunks.push(b);
          if(chunks.length>=1024)chunks=[Buffer.concat(chunks,total)];
        });
        r.on('error',()=>finish(new AuthError('probe_stream_interrupted')));
        r.on('aborted',()=>finish(new AuthError('probe_stream_interrupted')));
        r.on('end',()=>finish(null,{wire:Buffer.concat(chunks,total),headers_ms:Math.round(headers),first_body_ms:first===null?null:Math.round(first),stream_end_ms:Math.round(performance.now()-start),wire_bytes:total}));
      });
      req.on('error',()=>finish(new AuthError('probe_transport_error')));
      timer=setTimeout(()=>finish(new AuthError('probe_deadline')),DEADLINE);
      req.end(payload);
    }catch{finish(new AuthError('probe_transport_error'));}
  });
}
export function decoderPath(){
  const dir=path.dirname(new URL(import.meta.url).pathname);
  const exe=path.resolve(dir,'../../target/release/examples/responses_qualify');
  const m=fs.statSync(exe);require(m.isFile()&&(m.mode&0o111)!==0,'build_decoder_first');
  return exe;
}
export function decodeWire(exe,wire){
  const r=spawnSync(exe,[],{input:wire,encoding:'utf8',timeout:3000,maxBuffer:8192,env:{PATH:'/usr/bin:/bin',LANG:'C.UTF-8'},stdio:['pipe','pipe','pipe']});
  require(r.status===0&&typeof r.stdout==='string','probe_decoder_failed');
  return strictJson(r.stdout);
}
// Caller holds the existing exclusive vault lock throughout. The store
// creates a durable, non-reusable reservation before the first POST.
export async function qualify(store,available,{send=sendOnce,decode,now=()=>Date.now()}={}){
  require(!store.probeExists,'probe_already_attempted');
  const p=store.profile;
  catalogAllowed(p,Math.floor(now()/1000));
  require(!store.pendingRegistration&&p.expires_at>Math.floor(now()/1000)+60,'probe_grant_expiring');
  require(Array.isArray(available)&&available.some(m=>m.slug===MODEL),'probe_model_unavailable');
  require(typeof decode==='function','build_decoder_first');
  const pending=intent(p,now());
  store.reserveProbe(pending);
  let result;
  try{
    const r=await send(p.access_token);
    const metrics={headers_ms:r.headers_ms,first_body_ms:r.first_body_ms,stream_end_ms:r.stream_end_ms,wire_bytes:r.wire_bytes};
    let decoded;
    try{decoded=decode(r.wire);}catch{decoded=null;}
    result=resultRecord(pending,decoded,metrics);
    if(decoded?.terminal==='unvalidated'&&['Io','Limit','Framing','Json','Protocol','Unsupported','MissingTerminal','Usage','Cancelled'].includes(decoded.category))result.category=decoded.category;
  }catch(e){
    result=resultRecord(pending,null,{});
    result.category=e instanceof AuthError&&/^probe_(?:http_[1-5][0-9]{2}|content_type|wire_limit|stream_interrupted|transport_error|deadline)$/.test(e.message)?e.message:'probe_interrupted';
  }
  store.finishProbe(pending,result);
  // Do not print the account binding, prompt hash, raw text or provider IDs.
  const {binding:_,request_sha256:__,created_ms:___,version:____,...safe}=result;
  return safe;
}
