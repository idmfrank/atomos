#!/usr/bin/env node
// Explicit owner-run auth utility. Never calls the inference or models API.
import http from 'node:http';
import {spawnSync} from 'node:child_process';
import {strictJson} from './strict-json.mjs';
import {randomBytes} from 'node:crypto';
import {require,AuthError,DISCOVERY,ENDPOINTS,discovery,begin,callback,tokenRecord,refreshForm,refreshed,revokeForm,signedOut,planGranted,MODEL_CATALOG,catalog,catalogAllowed} from './protocol.mjs';
import {withStore} from './store.mjs';
const MAX=256*1024;
const ALLOWED=new Set([DISCOVERY,ENDPOINTS.jwks_uri,ENDPOINTS.token_endpoint,ENDPOINTS.revocation_endpoint]);
async function request(url,form=null,empty=false,bearer=null){
  require((ALLOWED.has(url)&&bearer===null||url===MODEL_CATALOG&&bearer!==null&&!form&&!empty)&&process.env.NODE_TLS_REJECT_UNAUTHORIZED!=='0','unsafe_endpoint_or_tls');
  const r=await fetch(url,{method:form?'POST':'GET',body:form??undefined,redirect:'error',signal:AbortSignal.timeout(15000),headers:form?{'content-type':'application/x-www-form-urlencoded'}:{accept:'application/json',...(bearer?{authorization:'Bearer '+bearer}:{})}});
  if(!r.ok){await r.body?.cancel();throw new AuthError('auth_http_'+r.status);}
  if(empty){require(r.status===200,'revocation_not_confirmed');for await(const b of r.body??[]){require(b.length===0,'revocation_not_confirmed');}return;}
  let n=0;const chunks=[];
  for await(const b of r.body){n+=b.length;require(n<=MAX,'auth_response_limit');chunks.push(Buffer.from(b));}
  return strictJson(new TextDecoder('utf-8',{fatal:true}).decode(Buffer.concat(chunks,n)));
}
async function signIn(store){
  require(!store.pendingRegistration,'registration_pending_manual_recovery');
  require(!store.profile||store.profile.status==='signed_out','sign_out_first');
  const cfg=discovery(await request(DISCOVERY));
  // User opens a local start link; authorization URL (possibly ID-token
  // hint) is never printed or passed through a shell.
  let attempt,start,settle,reject,started=false,done=false;
  const result=new Promise((ok,bad)=>{settle=ok;reject=bad;});
  // Attach early handler to prevent an unhandled rejection during listen.
  result.catch(()=>{});
  const reply=(res,status,body,location=null)=>{
    res.writeHead(status,{'content-type':'text/plain; charset=utf-8','cache-control':'no-store','referrer-policy':'no-referrer','x-content-type-options':'nosniff','content-security-policy':"default-src 'none'; frame-ancestors 'none'",connection:'close',...(location?{location}:{})});res.end(body);
  };
  const server=http.createServer({maxHeaderSize:16384,requestTimeout:5000,headersTimeout:5000},(req,res)=>{
    if(done||!attempt||req.method!=='GET'||req.headers.host!==new URL(attempt.redirect).host||req.socket.remoteAddress!=='127.0.0.1'){reply(res,400,'Invalid request.');return;}
    if(req.url===start&&!started){started=true;reply(res,303,'Continue with ChatGPT.',attempt.url);return;}
    if(!started){reply(res,404,'Not found.');return;}
    try{
      const c=callback(attempt,req.url);
      done=true;reply(res,200,'Callback received. Return to AtomOS for the validation result.');settle(c);
    }catch(e){
      reply(res,400,'Authorization not accepted. Return to AtomOS.');
      if(attempt.used){done=true;reject(e);}
    }
  });
  server.maxConnections=8;server.setTimeout(5000,s=>s.destroy());
  await new Promise((ok,bad)=>{server.once('error',bad);server.listen(0,'127.0.0.1',ok);});
  const timer=setTimeout(()=>reject(new AuthError('authorization_timeout')),300000);
  try{
    attempt=begin(store.host,server.address().port,store.profile);
    start='/continue/'+randomBytes(24).toString('base64url');
    console.log('Continue with ChatGPT: http://127.0.0.1:'+server.address().port+start);
    const c=await result;
    if(!store.profile)store.markRegistration(c.client_id);
    const t=await request(cfg.token_endpoint,c.form);
    const jwks=await request(cfg.jwks_uri);
    const record=tokenRecord(t,attempt,c.client_id,jwks,store.host);
    // If the provider grants identity only, preserve registration but not
    // usable tokens. Best effort remote revocation for a returned refresh token.
    if(!planGranted(record)&&typeof t.refresh_token==='string'){
      try{await request(cfg.revocation_endpoint,revokeForm({...record,refresh_token:t.refresh_token}),true);}
      catch{console.log('Remote revocation unconfirmed; disconnect in ChatGPT Settings.');}
    }
    store.save(record);
    store.finishRegistration();
    console.log(planGranted(record)?'Signed in; plan scope granted. Live inference remains disabled.':'Identity verified; plan permission incomplete. Live inference disabled.');
  }finally{clearTimeout(timer);server.closeAllConnections();await new Promise(ok=>server.close(ok));}
}
async function refresh(store){
  const p=store.profile;require(p&&planGranted(p),'no_active_grant');
  const cfg=discovery(await request(DISCOVERY));
  const form=refreshForm(p);
  // Persist pending state with no usable access token before sending a
  // rotating refresh. Ambiguous outcomes cannot resume old credentials.
  store.save({...signedOut(p),status:'pending'});
  try{
    const t=await request(cfg.token_endpoint,form);
    const jwks=t.id_token===undefined?null:await request(cfg.jwks_uri);
    store.save(refreshed(p,t,jwks));console.log('Renewed auth session; inference remains disabled.');
  }catch{console.log('Refresh unconfirmed; sign out locally and disconnect app in ChatGPT Settings before reauthorization.');throw new AuthError('refresh_unconfirmed');}
}
async function signOut(store){
  const p=store.profile;require(p,'no_registration');
  if(p.status==='signed_out'){console.log('Already signed out locally.');return;}
  // Persist pending first; retain refresh token in memory only for revoke.
  store.save({...signedOut(p),status:'pending'});
  let confirmed=false;
  try{if(p.refresh_token){const cfg=discovery(await request(DISCOVERY));await request(cfg.revocation_endpoint,revokeForm(p),true);confirmed=true;}}catch{}
  store.save(signedOut(p));
  console.log(confirmed?'Signed out; remote revocation confirmed.':'Signed out locally; remote revocation unconfirmed. Disconnect app in ChatGPT Settings.');
}
// Use the installed OS terminal password prompt; no secret argv/env,
// shell, agent fallback or cache. No noninteractive unlock.
function unlock(label){
  require(process.stdin.isTTY&&process.stderr.isTTY,'interactive_tty_required');
  const r=spawnSync('/usr/bin/systemd-ask-password',['--timeout=60','--silent',label],{stdio:['inherit','pipe','inherit'],timeout:65000,maxBuffer:2048});
  const out=r.stdout;
  require(r.status===0&&Buffer.isBuffer(out),'vault_prompt_failed');
  const end=out.length&&out[out.length-1]===10?out.length-1:out.length;
  const pass=Buffer.from(out.subarray(0,end));out.fill(0);
  require(pass.length>=20&&pass.length<=1024&&!pass.includes(0)&&!pass.includes(10)&&!pass.includes(13),'vault_passphrase_length');
  return pass;
}
async function models(store){
  const p=store.profile;catalogAllowed(p);
  const list=catalog(await request(MODEL_CATALOG,null,false,p.access_token));
  console.log(JSON.stringify({models:list,live_inference_enabled:false},null,2));
}
async function main(){
  const [cmd,dir,consent,...extra]=process.argv.slice(2);
  require(!extra.length&&['status','sign-in','refresh','sign-out','models'].includes(cmd)&&dir,'usage: node scripts/auth/atomos-auth.mjs status|sign-in|refresh|sign-out|models ABSOLUTE_PRIVATE_DIR [--confirm-eligible-auth-only]');
  require(cmd==='status'?consent===undefined:consent==='--confirm-eligible-auth-only','explicit_owner_confirmation_required');
  const pass=unlock('AtomOS vault passphrase (at least 20 characters):');
  try{
  await withStore(dir,pass,async store=>{
    if(cmd==='sign-in'&&!store.profile){
      const second=unlock('Confirm new AtomOS vault passphrase:');
      try{require(pass.equals(second),'vault_passphrase_mismatch');}finally{second.fill(0);}
    }
    if(cmd==='models')return models(store);
    if(cmd==='sign-in')return signIn(store);
    if(cmd==='refresh')return refresh(store);
    if(cmd==='sign-out')return signOut(store);
    const p=store.profile;
    console.log(JSON.stringify({registration:!!p,pending_registration:store.pendingRegistration,status:p?.status??'unregistered',plan_scope_granted:!!p&&planGranted(p),access_expired:p?.status==='active'?p.expires_at<=Math.floor(Date.now()/1000):null,live_inference_enabled:false}));
  });
  }finally{pass.fill(0);}
}
main().catch(e=>{console.error(e instanceof AuthError?e.message:'Authentication operation failed; inspect local permissions and documented recovery without sharing credentials.');process.exitCode=1;});
