import test from 'node:test';
import assert from 'node:assert/strict';
import {generateKeyPairSync,sign,createHash} from 'node:crypto';
import {hostId,validHost,begin,challenge,callback,discovery,ENDPOINTS,SCOPES,ISSUER,RESOURCE,verifyIdentity,tokenRecord,planGranted,refreshForm,refreshed,revokeForm,signedOut} from './protocol.mjs';
import {withStore as openStore} from './store.mjs';
import {strictJson} from './strict-json.mjs';
import {seal,unseal} from './vault.mjs';
import {catalog,catalogAllowed} from './protocol.mjs';
const pass=Buffer.from('synthetic only fixture passphrase 2026');
const withStore=(dir,action)=>openStore(dir,pass,action);
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
const {privateKey,publicKey}=generateKeyPairSync('rsa',{modulusLength:2048});
const jwks={keys:[{...publicKey.export({format:'jwk'}),kid:'synthetic',alg:'RS256',use:'sig'}]};
const client='oaiapp_synthetic', subject='synthetic-subject', now=1700000000;
const enc=v=>Buffer.from(JSON.stringify(v)).toString('base64url');
function jwt(claims,head={alg:'RS256',kid:'synthetic'}){
  const body=enc(head)+'.'+enc(claims);
  return body+'.'+sign('RSA-SHA256',Buffer.from(body),privateKey).toString('base64url');
}
const claims=(a,extra={})=>({iss:ISSUER,sub:subject,aud:[client],iat:now,exp:now+3600,nonce:a.nonce,...extra});
function tokens(a,extra={}){return {id_token:jwt(claims(a)),access_token:'synthetic-access',refresh_token:'synthetic-refresh',token_type:'Bearer',expires_in:3600,scope:SCOPES.join(' '),...extra};}
function profile(host=hostId()){const a=begin(host,1455);return tokenRecord(tokens(a),a,client,jwks,host,now);}
function cb(a,extra={}){return '/auth/callback?'+new URLSearchParams({state:a.state,code:'synthetic-code',client_id:client,...extra});}
function fixture(t){const d=fs.mkdtempSync(path.join(os.tmpdir(),'atomos-auth-'));fs.chmodSync(d,0o700);t.after(()=>fs.rmSync(d,{recursive:true,force:true}));return fs.realpathSync(d);}
test('RFC 7636 challenge and fresh authorization state',()=>{
  assert.equal(challenge('dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk'),'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM');
  const host=hostId();assert.ok(validHost(host));
  const a=begin(host,1455),b=begin(host,1455);
  for(const key of ['state','nonce','verifier']){assert.notEqual(a[key],b[key]);assert.equal(a[key].length,43);}
  const u=new URL(a.url);assert.equal(u.origin,ISSUER);
  assert.equal(u.searchParams.get('client_id'),'dynamic_agent_client');
  assert.equal(u.searchParams.get('agent_name_hint'),'AtomOS');
  assert.equal(u.searchParams.get('ext_agent_host_id'),host);
  assert.equal(u.searchParams.get('resource'),RESOURCE);
  assert.equal(u.searchParams.get('redirect_uri'),'http://127.0.0.1:1455/auth/callback');
  assert.equal(u.searchParams.get('code_challenge'),challenge(a.verifier));
  assert.equal(u.searchParams.has('code_verifier'),false);
});
test('discovery changes fail closed',()=>{
  const d={...ENDPOINTS,code_challenge_methods_supported:['S256'],id_token_signing_alg_values_supported:['RS256'],token_endpoint_auth_methods_supported:['none']};
  assert.deepEqual(discovery(d),ENDPOINTS);
  assert.throws(()=>discovery({...d,jwks_uri:'https://example.org/keys'}),/discovery_endpoint_changed/);
  assert.throws(()=>discovery({...d,id_token_signing_alg_values_supported:['none']}),/discovery_contract_changed/);
});
test('callback state, one-time use, exact redirect and issued ID',()=>{
  const a=begin(hostId(),49152);
  assert.throws(()=>callback(a,cb(a,{state:'wrong'})),/state_mismatch/);
  assert.throws(()=>callback(a,cb(a)+'&code=duplicate'),/duplicate_callback_field/);
  const c=callback(a,cb(a));
  assert.equal(c.form.get('client_id'),client);
  assert.equal(c.form.get('redirect_uri'),a.redirect);
  assert.equal(c.form.get('code_verifier'),a.verifier);
  assert.equal(c.form.has('client_secret'),false);
  assert.throws(()=>callback(a,cb(a)),/attempt_expired_or_used/);
  const b=begin(hostId(),1455,null,100);
  assert.throws(()=>callback(b,cb(b),300100),/attempt_expired_or_used/);
});
test('denial and registration mismatch do not exchange codes',()=>{
  const a=begin(hostId(),1455);
  assert.throws(()=>callback(a,'/auth/callback?'+new URLSearchParams({state:a.state,error:'access_denied'})),/authorization_denied/);
  assert.throws(()=>callback(a,cb(a)),/attempt_expired_or_used/);
  const b=begin(hostId(),1455,{client_id:client,subject});
  assert.equal(new URL(b.url).searchParams.get('agent_name_hint'),null);
  assert.throws(()=>callback(b,cb(b,{client_id:'oaiapp_other'})),/registration_mismatch/);
  const c=begin(hostId(),1455);
  assert.throws(()=>callback(c,cb(c,{client_id:'dynamic_agent_client'})),/missing_code_or_issued_id/);
});
test('RS256 validation rejects wrong signature, alg, key, issuer, audience, nonce, expiry and subject',()=>{
  const a=begin(hostId(),1455), good=jwt(claims(a));
  assert.equal(verifyIdentity(good,jwks,client,a.nonce,now).subject,subject);
  const [h,p,s]=good.split('.');
  for(const token of [h+'.'+enc(claims(a,{sub:'altered'}))+'.'+s,jwt(claims(a),{alg:'none',kid:'synthetic'}),jwt(claims(a),{alg:'RS256',kid:'unknown'}),jwt(claims(a,{iss:'https://example.org'})),jwt(claims(a,{aud:['oaiapp_other']})),jwt(claims(a,{nonce:'wrong'})),jwt(claims(a,{exp:now})),jwt(claims(a,{iat:now+60})),jwt(claims(a,{nbf:now+60}))]){
    assert.throws(()=>verifyIdentity(token,jwks,client,a.nonce,now));
  }
  assert.throws(()=>verifyIdentity(good,{keys:[jwks.keys[0],jwks.keys[0]]},client,a.nonce,now),/unknown_or_duplicate_key/);
  assert.throws(()=>verifyIdentity(good,jwks,client,a.nonce,now,'other'),/subject_mismatch/);
});
test('access-token hash and verified email are checked',()=>{
  const a=begin(hostId(),1455),access='fixture-access';
  const hash=createHash('sha256').update(access).digest().subarray(0,16).toString('base64url');
  const t=jwt(claims(a,{at_hash:hash,email:'fixture@example.invalid',email_verified:false}));
  assert.equal(verifyIdentity(t,jwks,client,a.nonce,now,null,access).email,null);
  assert.throws(()=>verifyIdentity(t,jwks,client,a.nonce,now,null,'wrong'),/access_hash_mismatch/);
});
test('token endpoint scopes, not callback scope, authorize plan use',()=>{
  const a=begin(hostId(),1455);
  const p=tokenRecord(tokens(a),a,client,jwks,a.host,now);
  assert.ok(planGranted(p));assert.equal(p.expires_at,now+3600);
  const denied=tokenRecord(tokens(a,{scope:'openid profile email'}),a,client,jwks,a.host,now);
  assert.equal(planGranted(denied),false);assert.equal(denied.access_token,undefined);
  assert.throws(()=>tokenRecord(tokens(a,{refresh_token:''}),a,client,jwks,a.host,now),/invalid_token_response/);
});
test('refresh rotates both tokens, retains identity, forbids scope escalation and subject switch',()=>{
  const p=profile();
  const f=refreshForm(p);
  assert.equal(f.get('client_id'),client);assert.equal(f.get('refresh_token'),p.refresh_token);assert.equal(f.has('scope'),false);
  const t={access_token:'next-access',refresh_token:'next-refresh',token_type:'Bearer',expires_in:3600};
  const q=refreshed(p,t,null,now+1);
  assert.equal(q.refresh_token,'next-refresh');assert.equal(q.access_token,'next-access');assert.equal(q.id_token,p.id_token);
  assert.throws(()=>refreshed(p,{...t,scope:'openid'},null,now+1),/refresh_scope_changed/);
  const a={nonce:p.nonce};
  assert.throws(()=>refreshed(p,{...t,id_token:jwt(claims(a,{sub:'other'}))},jwks,now+1),/subject_mismatch/);
  const rf=revokeForm(q);assert.equal(rf.get('token'),'next-refresh');assert.equal(rf.get('token_type_hint'),'refresh_token');
  assert.equal(signedOut(q).refresh_token,undefined);
});
test('protected atomic record, stable host, exclusive writer and preserved registration',async t=>{
  const dir=fixture(t);let first;
  await withStore(dir,async s=>{
    first=s.host;s.save(profile(first));
    assert.equal(fs.statSync(path.join(dir,'profile.json')).mode&0o777,0o600);
    assert.throws(()=>withStore(dir,()=>{}));
    assert.throws(()=>s.save({...s.profile,subject:'other'}),/profile_binding/);
  });
  await withStore(dir,s=>{assert.equal(s.host,first);assert.ok(planGranted(s.profile));s.save(signedOut(s.profile));});
  await withStore(dir,s=>{assert.equal(s.profile.status,'signed_out');assert.equal(s.profile.client_id,client);assert.equal(s.profile.access_token,undefined);});
});
test('unsafe files, missing host and unfinished registration fail closed',async t=>{
  const dir=fixture(t);
  await withStore(dir,s=>{s.markRegistration(client);});
  await withStore(dir,s=>{assert.ok(s.pendingRegistration);assert.throws(()=>s.markRegistration(client));s.finishRegistration();});
  fs.chmodSync(path.join(dir,'host.json'),0o644);
  await assert.rejects(withStore(dir,()=>{}),/unsafe_file/);
  fs.chmodSync(path.join(dir,'host.json'),0o600);
  fs.symlinkSync(path.join(dir,'host.json'),path.join(dir,'profile.json'));
  await assert.rejects(withStore(dir,()=>{}));
});
test('pending refresh discards reusable credentials across reopen',async t=>{
  const dir=fixture(t);
  await withStore(dir,s=>{s.save(profile(s.host));s.save({...signedOut(s.profile),status:'pending'});});
  await withStore(dir,s=>{assert.equal(s.profile.status,'pending');assert.equal(s.profile.refresh_token,undefined);assert.equal(planGranted(s.profile),false);});
});

test('strict JSON rejects escaped duplicate keys, nested duplicates and structural excess',()=>{
  assert.deepEqual(strictJson('{"a":[{"b":"\\\"x"}], "c":true}'),{a:[{b:'"x'}],c:true});
  for(const s of ['{"a":1,"a":2}','{"a":1,"\\u0061":2}','{"a":[{"b":1,"b":2}]}','['.repeat(34)+'0'+']'.repeat(34),'[1,]'])assert.throws(()=>strictJson(s));
});
test('encrypted profile has no plaintext secrets, rejects wrong key, tag, host and legacy record',async t=>{
  const dir=fixture(t);let saved;
  await withStore(dir,s=>{saved=profile(s.host);s.save(saved);});
  const file=path.join(dir,'profile.json'),raw=fs.readFileSync(file,'utf8');
  assert.ok(!raw.includes('synthetic-access')&&!raw.includes('synthetic-refresh')&&!raw.includes('synthetic-subject'));
  await assert.rejects(openStore(dir,Buffer.from('wrong synthetic fixture passphrase'),()=>{}),/vault_invalid_or_locked/);
  const envelope=JSON.parse(raw);
  assert.throws(()=>unseal(envelope,pass,hostId()),/vault_invalid_or_locked/);
  const tampered={...envelope,tag:Buffer.alloc(16).toString('base64url')};
  assert.throws(()=>unseal(tampered,pass,saved.host),/vault_invalid_or_locked/);
  assert.notEqual(seal(saved,pass,saved.host).data,envelope.data);
  fs.writeFileSync(file,JSON.stringify(saved));await assert.rejects(withStore(dir,()=>{}),/vault_invalid_or_locked/);
});
test('model catalog preserves listed ordering and denies duplicates, unsafe names and expired grants',()=>{
  const p=profile();
  assert.equal(catalogAllowed(p,now),true);
  assert.throws(()=>catalogAllowed(p,now+3600),/catalog_requires_current_grant/);
  assert.throws(()=>catalogAllowed(p,now-1),/catalog_requires_current_grant/);
  assert.throws(()=>catalogAllowed(signedOut(p),now),/catalog_requires_current_grant/);
  const a={visibility:'list',slug:'gpt-fixture',display_name:'Fixture'};
  const b={visibility:'hidden',slug:'hidden',display_name:'Hidden'};
  assert.deepEqual(catalog({models:[a,b,{...a,slug:'second'}]}),[{slug:'gpt-fixture',display_name:'Fixture'},{slug:'second',display_name:'Fixture'}]);
  assert.throws(()=>catalog({models:[a,a]}),/invalid_model_entry/);
  assert.throws(()=>catalog({models:[{...a,display_name:'bad\nname'}]}),/invalid_model_entry/);
  assert.throws(()=>catalog({data:[]}),/invalid_model_catalog/);
});
