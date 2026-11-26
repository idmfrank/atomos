// Auth-only cold path. No inference or provider bearer request.
import {randomBytes, randomUUID, createHash, timingSafeEqual, createPublicKey, verify} from 'node:crypto';
import {strictJson} from './strict-json.mjs';
export const ISSUER='https://auth.openai.com';
export const RESOURCE='https://api.openai.com/v1';
export const DISCOVERY=ISSUER+'/.well-known/openid-configuration';
export const ENDPOINTS=Object.freeze({
  issuer:ISSUER,authorization_endpoint:ISSUER+'/api/accounts/authorize',
  token_endpoint:ISSUER+'/api/accounts/oauth/token',
  revocation_endpoint:ISSUER+'/api/accounts/oauth/revoke',
  jwks_uri:ISSUER+'/.well-known/jwks.json'
});
export const SCOPES=['openid','profile','email','offline_access','resource.invoke','chatgpt.tokens.use.direct'];
const MAX_TOKEN=65536;
export class AuthError extends Error {}
export function require(ok,code){if(!ok)throw new AuthError(code);}
export function opaque(s,max=1024){return typeof s==='string'&&s.length>0&&s.length<=max&&/^[\x21-\x7e]+$/.test(s);}
export function clientId(s){return opaque(s,256)&&/^oaiapp_[A-Za-z0-9_-]+$/.test(s);}
export function hostId(){return 'urn:uuid:'+randomUUID();}
export function validHost(s){return typeof s==='string'&&/^urn:uuid:[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(s);}
function random(){return randomBytes(32).toString('base64url');}
export function challenge(verifier){return createHash('sha256').update(verifier,'ascii').digest('base64url');}
function eq(a,b){if(typeof a!=='string'||typeof b!=='string')return false;const x=Buffer.from(a),y=Buffer.from(b);return x.length===y.length&&timingSafeEqual(x,y);}
export function discovery(d){
  for(const [k,v] of Object.entries(ENDPOINTS))require(d?.[k]===v,'discovery_endpoint_changed');
  require(d?.code_challenge_methods_supported?.includes('S256')&&d?.id_token_signing_alg_values_supported?.includes('RS256')&&d?.token_endpoint_auth_methods_supported?.includes('none'),'discovery_contract_changed');
  return ENDPOINTS;
}
export function begin(host,port,previous=null,now=Date.now()){
  require(validHost(host)&&Number.isInteger(port)&&port>0&&port<=65535,'invalid_host_or_port');
  require(previous===null||clientId(previous.client_id),'invalid_registration');
  const a={host,previous,state:random(),nonce:random(),verifier:random(),redirect:'http://127.0.0.1:'+port+'/auth/callback',until:now+300000,used:false};
  const p=new URLSearchParams({client_id:previous?.client_id??'dynamic_agent_client',ext_agent_host_id:host,response_type:'code',redirect_uri:a.redirect,scope:SCOPES.join(' '),resource:RESOURCE,state:a.state,nonce:a.nonce,code_challenge_method:'S256',code_challenge:challenge(a.verifier)});
  if(!previous)p.set('agent_name_hint','AtomOS');
  else if(previous.id_token)p.set('id_token_hint',previous.id_token);
  if(previous&&!planGranted(previous))p.set('prompt','consent');
  a.url=ENDPOINTS.authorization_endpoint+'?'+p;
  return a;
}
export function callback(a,target,now=Date.now()){
  require(!a.used&&now<a.until,'attempt_expired_or_used');
  require(typeof target==='string'&&target.length<=16384&&target.startsWith('/auth/callback?')&&!target.includes('#'),'invalid_callback');
  const p=new URL(target,a.redirect).searchParams;
  for(const k of ['state','code','client_id','error','scope','iss'])require(p.getAll(k).length<=1,'duplicate_callback_field');
  require(eq(p.get('state'),a.state),'state_mismatch');
  if(p.has('iss'))require(p.get('iss')===ISSUER,'issuer_mismatch');
  a.used=true;
  if(p.has('error')){require(!p.has('code'),'ambiguous_callback');throw new AuthError('authorization_denied');}
  const code=p.get('code'),id=p.get('client_id')??a.previous?.client_id;
  require(opaque(code,8192)&&clientId(id),'missing_code_or_issued_id');
  require(!a.previous||id===a.previous.client_id,'registration_mismatch');
  // Callback scope is deliberately ignored: token endpoint grants authority.
  return {client_id:id,form:new URLSearchParams({grant_type:'authorization_code',client_id:id,code,code_verifier:a.verifier,redirect_uri:a.redirect,resource:RESOURCE})};
}
function segment(s){
  require(typeof s==='string'&&s.length<=MAX_TOKEN&&/^[A-Za-z0-9_-]+$/.test(s),'invalid_jwt');
  const b=Buffer.from(s,'base64url');
  require(b.toString('base64url')===s,'invalid_jwt_encoding');return b;
}
function object(s){
  try{const v=strictJson(new TextDecoder('utf-8',{fatal:true}).decode(segment(s)));require(v&&typeof v==='object'&&!Array.isArray(v),'invalid_jwt_object');return v;}
  catch{throw new AuthError('invalid_jwt_object');}
}
// Restricted RS256 profile from discovery; native OpenSSL via Node crypto.
// Never select algorithms or key URLs from untrusted JWT fields.
export function verifyIdentity(token,jwks,id,nonce,now=Math.floor(Date.now()/1000),subject=null,access=null){
  require(opaque(token,MAX_TOKEN)&&clientId(id),'invalid_identity_input');
  const parts=token.split('.');require(parts.length===3,'invalid_jwt');
  const [h,p,s]=parts,header=object(h);
  require(header.alg==='RS256'&&opaque(header.kid,128)&&!header.crit&&!header.jku&&!header.x5u&&!header.jwk,'unsupported_jwt_header');
  require(Array.isArray(jwks?.keys)&&jwks.keys.length<=32,'invalid_jwks');
  const keys=jwks.keys.filter(k=>k?.kid===header.kid);
  require(keys.length===1,'unknown_or_duplicate_key');
  const key=keys[0];
  require(key.kty==='RSA'&&key.alg==='RS256'&&key.use==='sig'&&(!key.key_ops||key.key_ops.includes('verify')),'unsupported_key');
  let valid=false;
  try{
    const pub=createPublicKey({key:{kty:'RSA',n:key.n,e:key.e},format:'jwk'});
    require(pub.asymmetricKeyDetails.modulusLength>=2048&&pub.asymmetricKeyDetails.modulusLength<=4096,'rsa_size');
    valid=verify('RSA-SHA256',Buffer.from(h+'.'+p,'ascii'),pub,segment(s));
  }catch{throw new AuthError('invalid_signature');}
  require(valid,'invalid_signature');
  const c=object(p);
  const aud=typeof c.aud==='string'?[c.aud]:c.aud;
  require(c.iss===ISSUER&&Array.isArray(aud)&&aud.length===1&&aud[0]===id&&(c.azp===undefined||c.azp===id),'identity_issuer_or_audience');
  require(Number.isSafeInteger(c.exp)&&Number.isSafeInteger(c.iat)&&c.exp>now&&c.iat<=now+5&&c.exp>c.iat&&c.exp-c.iat<=86400,'identity_expired_or_time');
  if(c.nbf!==undefined)require(Number.isSafeInteger(c.nbf)&&c.nbf<=now+5,'identity_not_yet_valid');
  if(nonce!==null)require(eq(c.nonce,nonce),'nonce_mismatch');
  else require(c.nonce===undefined,'unexpected_refresh_nonce');
  require(opaque(c.sub,1024)&&(!subject||c.sub===subject),'subject_mismatch');
  if(c.at_hash!==undefined){
    require(opaque(access,MAX_TOKEN),'missing_access_for_hash');
    require(eq(c.at_hash,createHash('sha256').update(access,'ascii').digest().subarray(0,16).toString('base64url')),'access_hash_mismatch');
  }
  return {subject:c.sub,email:c.email_verified===true&&typeof c.email==='string'&&c.email.length<=320?c.email:null};
}
export function scopes(s){
  require(typeof s==='string'&&s.length<=4096,'invalid_scopes');
  const list=s.split(' ').filter(Boolean);
  require(list.length<=64&&list.every(v=>/^[A-Za-z0-9._:-]+$/.test(v))&&new Set(list).size===list.length,'invalid_scopes');
  return list.sort();
}
export function planGranted(p){return p?.status==='active'&&Array.isArray(p.scopes)&&SCOPES.every(s=>p.scopes.includes(s));}
export function tokenRecord(t,attempt,id,jwks,host,now=Math.floor(Date.now()/1000)){
  require(t&&typeof t==='object'&&opaque(t.id_token,MAX_TOKEN),'missing_identity_token');
  const identity=verifyIdentity(t.id_token,jwks,id,attempt.nonce,now,attempt.previous?.subject??null,t.access_token);
  const granted=scopes(t.scope??'');
  const base={version:1,host,client_id:id,subject:identity.subject,email:identity.email,scopes:granted,status:'signed_out',saved_at:now};
  if(!SCOPES.every(s=>granted.includes(s))){
    // Identity alone never creates an active plan grant. Discard tokens;
    // remote grant cleanup is a separate explicit operation.
    return base;
  }
  require(typeof t.token_type==='string'&&t.token_type.toLowerCase()==='bearer'&&opaque(t.access_token,MAX_TOKEN)&&opaque(t.refresh_token,MAX_TOKEN)&&Number.isSafeInteger(t.expires_in)&&t.expires_in>0&&t.expires_in<=86400,'invalid_token_response');
  return {...base,status:'active',access_token:t.access_token,refresh_token:t.refresh_token,id_token:t.id_token,nonce:attempt.nonce,expires_at:now+t.expires_in};
}
export function refreshForm(p){require(planGranted(p)&&opaque(p.refresh_token,MAX_TOKEN),'refresh_unavailable');return new URLSearchParams({grant_type:'refresh_token',client_id:p.client_id,refresh_token:p.refresh_token,resource:RESOURCE});}
export function refreshed(p,t,jwks,now=Math.floor(Date.now()/1000)){
  require(t&&typeof t==='object'&&typeof t.token_type==='string'&&t.token_type.toLowerCase()==='bearer'&&opaque(t.access_token,MAX_TOKEN)&&opaque(t.refresh_token,MAX_TOKEN)&&Number.isSafeInteger(t.expires_in)&&t.expires_in>0&&t.expires_in<=86400,'invalid_refresh_response');
  const granted=t.scope===undefined?p.scopes:scopes(t.scope);
  require(granted.every(s=>p.scopes.includes(s))&&SCOPES.every(s=>granted.includes(s)),'refresh_scope_changed');
  if(t.id_token!==undefined){
    const claims=object(t.id_token.split('.')[1]);
    verifyIdentity(t.id_token,jwks,p.client_id,claims.nonce===undefined?null:p.nonce,now,p.subject,t.access_token);
  }
  return {...p,status:'active',scopes:granted,access_token:t.access_token,refresh_token:t.refresh_token,id_token:t.id_token??p.id_token,saved_at:now,expires_at:now+t.expires_in};
}
export function revokeForm(p){require(opaque(p.refresh_token,MAX_TOKEN)&&clientId(p.client_id),'revocation_unavailable');return new URLSearchParams({token:p.refresh_token,token_type_hint:'refresh_token',client_id:p.client_id});}
export function signedOut(p){return {version:1,host:p.host,client_id:p.client_id,subject:p.subject,email:p.email,scopes:[],status:'signed_out',saved_at:p.saved_at};}

export const MODEL_CATALOG=RESOURCE+'/models';
export function catalog(value){
  require(Array.isArray(value?.models)&&value.models.length<=256,'invalid_model_catalog');
  const out=[],seen=new Set();
  for(const m of value.models){
    require(m&&typeof m==='object'&&!Array.isArray(m)&&typeof m.visibility==='string','invalid_model_entry');
    if(m.visibility!=='list')continue;
    require(typeof m.slug==='string'&&/^[A-Za-z0-9._:-]{1,128}$/.test(m.slug)&&typeof m.display_name==='string'&&m.display_name.length>0&&m.display_name.length<=128&&!/[\p{Cc}\p{Cf}]/u.test(m.display_name)&&!seen.has(m.slug),'invalid_model_entry');
    seen.add(m.slug);out.push({slug:m.slug,display_name:m.display_name});
  }
  return out;
}
export function catalogAllowed(p,now=Math.floor(Date.now()/1000)){
  require(planGranted(p)&&opaque(p.access_token,65536)&&Number.isSafeInteger(p.saved_at)&&Number.isSafeInteger(p.expires_at)&&now>=p.saved_at&&p.expires_at>now+5,'catalog_requires_current_grant');
  return true;
}
