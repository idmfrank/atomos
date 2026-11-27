// Explicit one-attempt qualification only; never a general inference API.
import https from 'node:https';
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {performance} from 'node:perf_hooks';
import {gunzipSync} from 'node:zlib';
import {AuthError,require,catalogAllowed,RESOURCE} from './protocol.mjs';
import {strictJson} from './strict-json.mjs';

export const MODEL='gpt-6.2-sol';
export const PROMPT='Say exactly: Hello, world!';
export const ACK='--confirm-personal-app-below-100-and-uncapped-single-request';
export const FOLLOWUP_ACK='--confirm-personal-app-below-100-second-uncapped-request-retain-unknown-usage';
export const COMPAT_ACK='--confirm-personal-app-below-100-final-uncapped-request-retain-two-unknown-attempts';
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
// Classify only allowlisted header categories. Never retain raw headers,
// provider IDs, response bodies or arbitrary error messages in diagnostics.
function mediaType(v){
  if(v===undefined)return 'missing';
  if(typeof v!=='string')return 'other';
  const type=v.split(';',1)[0].trim().toLowerCase();
  if(type==='text/event-stream')return 'sse';
  if(type==='application/json'||type==='application/problem+json')return 'json';
  if(type==='text/html')return 'html';
  if(type==='text/plain')return 'plain';
  return 'other';
}
function encodingType(v){
  if(v===undefined)return 'missing';
  if(typeof v!=='string')return 'other';
  const e=v.trim().toLowerCase();
  return ['identity','gzip','br','deflate'].includes(e)?e:'other';
}
class ProbeTransportError extends AuthError{
  constructor(code,metrics){super(code);this.metrics=metrics;}
}
// Exactly one HTTP request, no redirect handling, agent reuse, retry,
// SDK or API-key fallback. Absolute wall deadline includes connection/body.
// Gzip is optional; both received and decompressed bodies have independent
// 256 KiB ceilings. No MIME sniffing or acceptance of JSON as SSE.
export function sendOnce(access,request=https.request){
  return transport(access,request,false);
}
// Only selected by the separately gated compatibility qualification.
function sendCompatibleOnce(access){return transport(access,https.request,true);}
// Offline experiment only. Fixed synthetic token and numeric loopback address;
// cannot target a provider, accept real credentials or be selected by the CLI.
export async function replayLoopbackCandidate(port){
  require(Number.isInteger(port)&&port>=1&&port<=65535,'invalid_fixture_port');
  const request=(_url,options,callback)=>http.request({hostname:'127.0.0.1',port,path:'/v1/responses',method:options.method,agent:false,headers:options.headers},callback);
  const r=await transport('fixture-access',request,true);
  const {wire,...metrics}=r;
  return {metrics,decoded:decodeWire(decoderPath(),wire)};
}
function transport(access,request,allowMissingMedia){
  return new Promise((resolve,reject)=>{
    const payload=body(),start=performance.now();
    let timer,finished=false,req,res,first=null,headers=null,total=0,chunks=[];
    let status=null,media=null,encoding=null;
    const metrics=()=>({http_status:status,content_type_class:media,content_encoding_class:encoding,headers_ms:headers===null?null:Math.round(headers),first_body_ms:first===null?null:Math.round(first),wire_bytes:total,elapsed_ms:Math.round(performance.now()-start)});
    const finish=(code,value)=>{
      if(finished)return;finished=true;clearTimeout(timer);
      if(code){const m=metrics();res?.destroy();req?.destroy();reject(new ProbeTransportError(code,m));}else resolve(value);
    };
    try{
      require(process.env.NODE_TLS_REJECT_UNAUTHORIZED!=='0','unsafe_tls');
      req=request(ENDPOINT,{method:'POST',agent:false,rejectUnauthorized:true,headers:{authorization:'Bearer '+access,'content-type':'application/json','accept':'text/event-stream','accept-encoding':'gzip, identity','content-length':Buffer.byteLength(payload)}},r=>{
        if(finished){r.destroy();return;}
        res=r;headers=performance.now()-start;
        status=integer(r.statusCode,599)&&r.statusCode>=100?r.statusCode:null;
        media=mediaType(r.headers['content-type']);
        encoding=encodingType(r.headers['content-encoding']);
        r.on('error',()=>finish('probe_stream_interrupted'));
        r.on('aborted',()=>finish('probe_stream_interrupted'));
        if(status!==200){finish(status===null?'probe_transport_error':'probe_http_'+status);return;}
        if(media!=='sse'&&!(allowMissingMedia&&media==='missing')){finish('probe_content_type');return;}
        if(!['missing','identity','gzip'].includes(encoding)){finish('probe_content_encoding');return;}
        r.on('data',b=>{
          if(finished)return;
          if(first===null)first=performance.now()-start;
          total+=b.length;
          if(total>MAX){finish('probe_wire_limit');return;}
          chunks.push(b);
          if(chunks.length>=1024)chunks=[Buffer.concat(chunks,total)];
        });
        r.on('end',()=>{
          if(finished)return;
          const ended=performance.now()-start;
          let wire=Buffer.concat(chunks,total);
          if(encoding==='gzip'){
            try{wire=gunzipSync(wire,{maxOutputLength:MAX});}
            catch(e){finish(e?.code==='ERR_BUFFER_TOO_LARGE'?'probe_decoded_limit':'probe_invalid_compression');return;}
          }
          if(wire.length>MAX){finish('probe_decoded_limit');return;}
          if(performance.now()-start>DEADLINE){finish('probe_deadline');return;}
          finish(null,{wire,...metrics(),stream_end_ms:Math.round(ended),decoded_bytes:wire.length});
        });
      });
      req.on('error',()=>finish('probe_transport_error'));
      timer=setTimeout(()=>finish('probe_deadline'),DEADLINE);
      req.end(payload);
    }catch{finish('probe_transport_error');}
  });
}
export function decoderPath(){
  const dir=path.dirname(fileURLToPath(import.meta.url));
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
export function followupEligible(store,now=Date.now()){
  require(!store.followupExists&&!store.compatExists,'probe_followup_already_attempted');
  require(store.probeExists,'probe_original_missing');
  const first=store.originalProbe();
  const p=store.profile;
  // The only allowed follow-up is the specific historical v1 combined
  // content-type/encoding failure. No quota/auth/timeout or generic retry.
  require(p&&first?.version===1&&first.state==='unknown'&&first.outcome==='unvalidated'&&first.category==='probe_content_type'&&first.binding===binding(p)&&first.request_sha256===hash(body())&&first.model===MODEL&&Number.isSafeInteger(first.created_ms)&&first.created_ms>0&&first.created_ms<=now&&first.reserved_planning_tokens===RESERVED&&first.charged_planning_units===RESERVED&&first.reported_tokens===null&&first.unknown_usage===true&&first.expected_text===null&&!Object.hasOwn(first,'http_status'),'probe_original_not_eligible');
  return first;
}
export async function qualify(store,available,options={}){
  require(!store.probeExists&&!store.followupExists&&!store.compatExists,'probe_already_attempted');
  return runQualification(store,available,options,null);
}
export async function qualifyFollowup(store,available,options={}){
  const first=followupEligible(store,(options.now??Date.now)());
  return runQualification(store,available,options,first);
}
// Match only the two documented historical unknown header failures. Reuse
// the original identity, request and immutable records; no generic retry.
export function compatEligible(store,now=Date.now()){
  require(!store.compatExists,'probe_compat_already_attempted');
  require(store.probeExists&&store.followupExists,'probe_compat_history_missing');
  const first=store.originalProbe(),second=store.followupProbe(),p=store.profile;
  const unknown=r=>r?.version===1&&r.state==='unknown'&&r.outcome==='unvalidated'&&r.category==='probe_content_type'&&r.binding===binding(p)&&r.request_sha256===hash(body())&&r.model===MODEL&&Number.isSafeInteger(r.created_ms)&&r.created_ms>0&&r.created_ms<=now&&r.reserved_planning_tokens===RESERVED&&r.charged_planning_units===RESERVED&&r.reported_tokens===null&&r.unknown_usage===true&&r.expected_text===null;
  require(p&&unknown(first)&&!Object.hasOwn(first,'http_status')&&unknown(second)&&second.created_ms>=first.created_ms&&second.attempt===2&&second.previous_record_sha256===hash(JSON.stringify(first))&&second.prior_retained_planning_units===RESERVED&&second.cumulative_planning_units===2*RESERVED&&second.prior_usage_unknown===true&&second.http_status===200&&second.content_type_class==='missing'&&second.content_encoding_class==='missing'&&second.first_body_ms===null&&second.wire_bytes===0,'probe_compat_history_ineligible');
  return {first,second};
}
export async function qualifyCompat(store,available,options={}){
  const {first,second}=compatEligible(store,(options.now??Date.now)());
  return runQualification(store,available,{send:sendCompatibleOnce,...options},first,second);
}
function safeDiagnostics(d){
  if(!d||!['initial','created','queued','in_progress','item','part','text_done','part_done','item_done','terminal'].includes(d.stage)||!['none','unparsed','response.created','response.in_progress','response.queued','response.output_text.annotation.added','response.output_item.added','response.output_item.done','response.content_part.added','response.content_part.done','response.output_text.delta','response.output_text.done','response.completed','response.failed','response.incomplete','error','reasoning_event','refusal_event','other'].includes(d.event_class)||!['none','missing','message','reasoning','output_text','refusal','function_call','other'].includes(d.item_class)||!integer(d.validated_events,128))return null;
  return {stage:d.stage,event_class:d.event_class,item_class:d.item_class,validated_events:d.validated_events};
}
async function runQualification(store,available,{send=sendOnce,decode,now=()=>Date.now()}={},first,second=null){


  const p=store.profile;
  catalogAllowed(p,Math.floor(now()/1000));
  require(!store.pendingRegistration&&p.expires_at>Math.floor(now()/1000)+60,'probe_grant_expiring');
  require(Array.isArray(available)&&available.some(m=>m.slug===MODEL),'probe_model_unavailable');
  require(typeof decode==='function','build_decoder_first');
  const pending=intent(p,now());
  if(second){
    Object.assign(pending,{attempt:3,original_record_sha256:hash(JSON.stringify(first)),previous_record_sha256:hash(JSON.stringify(second)),prior_retained_planning_units:2*RESERVED});
    store.reserveCompat(pending,first,second);
  }else if(first){
    Object.assign(pending,{attempt:2,previous_record_sha256:hash(JSON.stringify(first)),prior_retained_planning_units:RESERVED});
    store.reserveFollowup(pending,first);
  }else store.reserveProbe(pending);
  let result;
  try{
    const r=await send(p.access_token);
    const metrics={http_status:r.http_status,content_type_class:r.content_type_class,content_encoding_class:r.content_encoding_class,headers_ms:r.headers_ms,first_body_ms:r.first_body_ms,stream_end_ms:r.stream_end_ms,wire_bytes:r.wire_bytes,decoded_bytes:r.decoded_bytes,elapsed_ms:r.elapsed_ms};
    let decoded;
    try{decoded=decode(r.wire);}catch{decoded=null;}
    result=resultRecord(pending,decoded,metrics);
    const diagnostic=safeDiagnostics(decoded?.diagnostics);
    if(diagnostic)result.decoder_diagnostics=diagnostic;
    if(decoded===null)result.category='probe_decoder_failed';
    if(decoded?.terminal==='unvalidated'&&['Io','Limit','Framing','Json','Protocol','Unsupported','MissingTerminal','Usage','Cancelled'].includes(decoded.category))result.category=decoded.category;
  }catch(e){
    result=resultRecord(pending,null,e instanceof ProbeTransportError?e.metrics:{});
    result.category=e instanceof AuthError&&/^probe_(?:http_[1-5][0-9]{2}|content_type|content_encoding|wire_limit|decoded_limit|invalid_compression|stream_interrupted|transport_error|deadline)$/.test(e.message)?e.message:'probe_interrupted';
  }
  if(second){
    result.cumulative_planning_units=2*RESERVED+result.charged_planning_units;
    result.prior_usage_unknown=true;
    store.finishCompat(pending,result,first,second);
  }else if(first){
    result.cumulative_planning_units=RESERVED+result.charged_planning_units;
    result.prior_usage_unknown=true;
    store.finishFollowup(pending,result,first);
  }else store.finishProbe(pending,result);
  // Do not print the account binding, prompt hash, raw text or provider IDs.
  const {binding:_,request_sha256:__,created_ms:___,version:____,previous_record_sha256:_____,original_record_sha256:______,...safe}=result;
  return safe;
}
