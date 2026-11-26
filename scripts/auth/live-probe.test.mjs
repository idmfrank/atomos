import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {withStore} from './store.mjs';
import {SCOPES,hostId} from './protocol.mjs';
import {MODEL,body,qualify,intent,resultRecord,decodeWire,decoderPath} from './live-probe.mjs';
const now=1700000000000;
function profile(host=hostId()){return {version:1,host,client_id:'oaiapp_fixture',subject:'fixture',status:'active',scopes:SCOPES,access_token:'fixture-access',refresh_token:'fixture-refresh',id_token:'fixture-id',nonce:'fixture-nonce',saved_at:now/1000,expires_at:now/1000+3600};}
const available=[{slug:MODEL}];
const usage={input:35,output:12,cached_input:10,reasoning_output:4};
const decoded={terminal:'completed',usage,expected_text:true,text_bytes:13};
const pass=Buffer.from('synthetic fixture only passphrase');
function fixture(t){const d=fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(),'atomos-probe-')));t.after(()=>fs.rmSync(d,{recursive:true,force:true}));return d;}
test('exact request body contains no context, tools or unsupported fields',()=>{
  assert.deepEqual(JSON.parse(body()),{model:MODEL,input:[{role:'user',content:'Say exactly: Hello, world!'}],store:false,stream:true});
});
test('reserve durable before send, settle actual once and refuse replay after reopen',async t=>{
  const dir=fixture(t);let calls=0;
  await withStore(dir,pass,async s=>{
    s.save(profile(s.host));
    const r=await qualify(s,available,{now:()=>now,send:async access=>{
      assert.equal(access,'fixture-access');calls++;
      const record=JSON.parse(fs.readFileSync(path.join(dir,'m2-4-probe.json')));
      assert.equal(record.state,'pending');assert.equal(record.reserved_planning_tokens,1024);
      return {wire:Buffer.from('fixture'),headers_ms:1,first_body_ms:2,stream_end_ms:3,wire_bytes:7};
    },decode:()=>decoded});
    assert.equal(r.outcome,'qualified');assert.equal(r.charged_planning_units,47);assert.equal(r.binding,undefined);
  });
  await withStore(dir,pass,async s=>assert.rejects(qualify(s,available,{now:()=>now,send:async()=>{calls++;},decode:()=>decoded}),/probe_already_attempted/));
  assert.equal(calls,1);
  const raw=fs.readFileSync(path.join(dir,'m2-4-probe.json'),'utf8');assert.ok(!raw.includes('fixture-access')&&!raw.includes('fixture-refresh')&&!raw.includes('Hello, world!'));
});
test('expired or missing model refuses before reservation or dispatch',async t=>{
  const dir=fixture(t);
  await withStore(dir,pass,async s=>{
    const p=profile(s.host);s.save(p);
    const opts={now:()=>now,send:async()=>assert.fail('must not dispatch'),decode:()=>decoded};
    await assert.rejects(qualify(s,[],opts),/probe_model_unavailable/);
    await assert.rejects(qualify(s,available,{...opts,now:()=>now+3600000}),/catalog_requires_current_grant/);
    assert.equal(s.probeExists,false);
  });
});
test('interrupted attempt retains reservation and blocks replay',async t=>{
  const dir=fixture(t);
  await withStore(dir,pass,async s=>{
    s.save(profile(s.host));
    const r=await qualify(s,available,{now:()=>now,send:async()=>{throw Error('fixture secret');},decode:()=>decoded});
    assert.equal(r.state,'unknown');assert.equal(r.charged_planning_units,1024);assert.equal(r.category,'probe_interrupted');
  });
  await withStore(dir,pass,async s=>assert.rejects(qualify(s,available,{now:()=>now,decode:()=>decoded,send:async()=>assert.fail('replay')}),/probe_already_attempted/));
});
test('pending crash marker denies new attempt; invalid usage unknown, overrun settles actual',async t=>{
  const dir=fixture(t),p=profile();
  const pending=intent(p,now);
  assert.equal(resultRecord(pending,{...decoded,usage:{...usage,cached_input:36}},{}).state,'unknown');
  const r=resultRecord(pending,{...decoded,usage:{...usage,output:2000}},{});
  assert.equal(r.outcome,'overrun');assert.equal(r.charged_planning_units,2035);
  const f=resultRecord(pending,{terminal:'failed',category:'RateLimited',usage},{});
  assert.equal(f.outcome,'provider_failed');assert.equal(f.charged_planning_units,47);
  await withStore(dir,pass,async s=>{s.save(profile(s.host));s.reserveProbe(intent(s.profile,now));});
  await withStore(dir,pass,async s=>assert.rejects(qualify(s,available,{now:()=>now,decode:()=>decoded,send:async()=>assert.fail('replay')}),/probe_already_attempted/));
});
test('existing Rust decoder replays fixture without exposing output or secrets',()=>{
  const exe=decoderPath();
  const wire=fs.readFileSync(new URL('../../crates/atomos-core/tests/fixtures/responses-text.sse',import.meta.url));
  const d=decodeWire(exe,wire);assert.equal(d.terminal,'completed');assert.deepEqual(d.usage,usage);
  assert.equal(d.expected_text,false);assert.equal('text' in d,false);
  assert.equal(decodeWire(exe,Buffer.from('event: junk\n\n')).terminal,'unvalidated');
});
