// Single local registration. Protected files, not encrypted storage.
import fs from 'node:fs';
import path from 'node:path';
import {randomBytes} from 'node:crypto';
import {require,validHost,hostId,clientId,opaque,planGranted} from './protocol.mjs';
import {strictJson} from './strict-json.mjs';
import {seal,unseal} from './vault.mjs';
const MAX=512*1024;
function privateDir(dir){
  const m=fs.lstatSync(dir);
  require(m.isDirectory()&&m.uid===process.getuid()&&(m.mode&0o077)===0,'private_directory_required');
}
function openRead(file){
  const fd=fs.openSync(file,fs.constants.O_RDONLY|fs.constants.O_NOFOLLOW|fs.constants.O_NONBLOCK);
  try{
    const m=fs.fstatSync(fd);require(m.isFile()&&m.nlink===1&&m.uid===process.getuid()&&(m.mode&0o077)===0&&m.size<=MAX,'unsafe_file');
    const b=Buffer.alloc(MAX+1);let n=0;
    while(n<b.length){const k=fs.readSync(fd,b,n,b.length-n,null);if(k===0)break;n+=k;}
    require(n<=MAX,'oversize_file');
    return strictJson(new TextDecoder('utf-8',{fatal:true}).decode(b.subarray(0,n)));
  }finally{fs.closeSync(fd);}
}
function syncDir(dir){const fd=fs.openSync(dir,'r');try{fs.fsyncSync(fd);}finally{fs.closeSync(fd);}}
function atomic(dir,name,value){
  const dest=path.join(dir,name),tmp=path.join(dir,'.write-'+randomBytes(16).toString('hex'));
  const bytes=Buffer.from(JSON.stringify(value)+'\n');require(bytes.length<=MAX,'oversize_record');
  let fd;
  try{
    fd=fs.openSync(tmp,'wx',0o600);fs.writeFileSync(fd,bytes);fs.fsyncSync(fd);fs.closeSync(fd);fd=undefined;
    fs.renameSync(tmp,dest);syncDir(dir);
  }finally{if(fd!==undefined)fs.closeSync(fd);try{fs.unlinkSync(tmp);}catch(e){if(e.code!=='ENOENT')throw e;}}
}
function exists(p){try{fs.lstatSync(p);return true;}catch(e){if(e.code==='ENOENT')return false;throw e;}}
export function withStore(dir,pass,action){ // synchronous acquisition, async lifetime
  require(process.platform==='linux'&&path.isAbsolute(dir)&&fs.realpathSync(dir)===dir,'canonical_linux_directory_required');
  privateDir(dir);
  require(Buffer.isBuffer(pass)&&pass.length>=20&&pass.length<=1024,'vault_passphrase_required');
  const lock=path.join(dir,'writer.lock');fs.mkdirSync(lock,{mode:0o700});
  try{syncDir(dir);}catch(e){fs.rmdirSync(lock);throw e;}
  return (async()=>{
    try{
      const hp=path.join(dir,'host.json'),pp=path.join(dir,'profile.json');
      if(!exists(hp)){
        require(!exists(pp)&&!exists(path.join(dir,'registration-pending.json')),'host_missing');
        atomic(dir,'host.json',{host:hostId()});
      }
      const host=openRead(hp).host;require(validHost(host),'invalid_host');
      let profile=exists(pp)?unseal(openRead(pp),pass,host):null;
      if(profile)require(profile.version===1&&profile.host===host&&clientId(profile.client_id)&&opaque(profile.subject,1024)&&['active','signed_out','pending'].includes(profile.status)&&Array.isArray(profile.scopes),'invalid_profile');
      if(profile?.status==='active')require(planGranted(profile)&&opaque(profile.access_token,65536)&&opaque(profile.refresh_token,65536)&&opaque(profile.id_token,65536)&&opaque(profile.nonce,128)&&Number.isSafeInteger(profile.expires_at),'invalid_active_profile');
      const pending=path.join(dir,'registration-pending.json');
      const store={host,get profile(){return profile;},get pendingRegistration(){return exists(pending);},markRegistration(id){require(clientId(id)&&!exists(pending),'registration_pending');atomic(dir,'registration-pending.json',{client_id:id});},finishRegistration(){if(exists(pending)){fs.unlinkSync(pending);syncDir(dir);}},save(p){require(p.host===host&&clientId(p.client_id)&&opaque(p.subject,1024)&&(!profile||(p.client_id===profile.client_id&&p.subject===profile.subject)),'profile_binding');atomic(dir,'profile.json',seal(p,pass,host));profile=p;}};
      return await action(store);
    }finally{fs.rmdirSync(lock);syncDir(dir);}
  })();
}
