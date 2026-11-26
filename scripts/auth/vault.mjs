// Owner-unlocked, encrypted session record. Native OpenSSL primitives.
// Fixed KDF parameters; no attacker-controlled work factor or plaintext fallback.
import {randomBytes,scryptSync,createCipheriv,createDecipheriv} from 'node:crypto';
import {strictJson} from './strict-json.mjs';
const MAX=256*1024;
function check(ok){if(!ok)throw Error('vault_invalid_or_locked');}
function decode(s,min,max){
  check(typeof s==='string'&&s.length<=Math.ceil(max*4/3)&&/^[A-Za-z0-9_-]+$/.test(s));
  const b=Buffer.from(s,'base64url');check(b.length>=min&&b.length<=max&&b.toString('base64url')===s);return b;
}
function key(pass,salt){
  check(Buffer.isBuffer(pass)&&pass.length>=20&&pass.length<=1024);
  return scryptSync(pass,salt,32,{N:65536,r:8,p:1,maxmem:96*1024*1024});
}
function aad(host){return Buffer.from('AtomOS auth vault v1\0'+host,'utf8');}
export function seal(profile,pass,host){
  const plaintext=Buffer.from(JSON.stringify(profile),'utf8');check(plaintext.length<=MAX);
  const salt=randomBytes(16),iv=randomBytes(12),k=key(pass,salt);
  try{
    const c=createCipheriv('aes-256-gcm',k,iv);c.setAAD(aad(host));
    const data=Buffer.concat([c.update(plaintext),c.final()]);
    return {version:1,kdf:'scrypt-65536-8-1',cipher:'aes-256-gcm',salt:salt.toString('base64url'),iv:iv.toString('base64url'),tag:c.getAuthTag().toString('base64url'),data:data.toString('base64url')};
  }finally{k.fill(0);plaintext.fill(0);}
}
export function unseal(v,pass,host){
  check(v&&v.version===1&&v.kdf==='scrypt-65536-8-1'&&v.cipher==='aes-256-gcm'&&Object.keys(v).length===7);
  const salt=decode(v.salt,16,16),iv=decode(v.iv,12,12),tag=decode(v.tag,16,16),data=decode(v.data,1,MAX);
  const k=key(pass,salt);let first,tail,plaintext;
  try{
    const d=createDecipheriv('aes-256-gcm',k,iv);d.setAAD(aad(host));d.setAuthTag(tag);
    first=d.update(data);tail=d.final();plaintext=Buffer.concat([first,tail]);
    return strictJson(new TextDecoder('utf-8',{fatal:true}).decode(plaintext));
  }catch{throw Error('vault_invalid_or_locked');}
  finally{k.fill(0);first?.fill(0);tail?.fill(0);plaintext?.fill(0);}
}
