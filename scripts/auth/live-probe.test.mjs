import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {withStore} from './store.mjs';
import {SCOPES,hostId} from './protocol.mjs';
import {MODEL,body,qualify,intent,resultRecord,decodeWire,decoderPath,sendOnce,qualifyFollowup,followupEligible} from './live-probe.mjs';
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

test('transport sends one fixed HTTPS POST and rejects redirects without following',async()=>{
  const {EventEmitter}=await import('node:events');
  let calls=0;
  const request=(url,opts,cb)=>{
    calls++;assert.equal(url,'https://api.openai.com/v1/responses');
    assert.equal(opts.method,'POST');assert.equal(opts.agent,false);assert.equal(opts.rejectUnauthorized,true);
    assert.equal(opts.headers.authorization,'Bearer fixture-access');
    const req=new EventEmitter();req.destroy=()=>{};
    req.end=payload=>{assert.equal(payload,body());
      queueMicrotask(()=>{const r=new EventEmitter();r.statusCode=302;r.headers={location:'https://example.invalid'};r.destroy=()=>{};cb(r);});
    };return req;
  };
  await assert.rejects(sendOnce('fixture-access',request),/probe_http_302/);assert.equal(calls,1);
});
test('transport bounds body and accepts complete SSE without exposing token',async()=>{
  const {EventEmitter}=await import('node:events');
  function fake(chunks,ctype='text/event-stream'){
    return (_url,_opts,cb)=>{
      const req=new EventEmitter();req.destroy=()=>{};
      req.end=()=>queueMicrotask(()=>{
        const r=new EventEmitter();r.statusCode=200;r.headers={'content-type':ctype};r.destroy=()=>{};
        cb(r);for(const b of chunks)r.emit('data',b);r.emit('end');
      });return req;
    };
  }
  const r=await sendOnce('fixture-access',fake([Buffer.from('a'),Buffer.from('bc')]));
  assert.equal(r.wire.toString(),'abc');assert.equal(r.wire_bytes,3);assert.ok(r.first_body_ms!==null);
  await assert.rejects(sendOnce('fixture-access',fake([Buffer.alloc(256*1024+1)])),/probe_wire_limit/);
  await assert.rejects(sendOnce('fixture-access',fake([],'application/json')),/probe_content_type/);
});


test('transport classifies rejected headers without storing raw values or bodies',async()=>{
  const {EventEmitter}=await import('node:events');
  const request=(headers,statusCode=200)=> (_url,_opts,cb)=>{
    const req=new EventEmitter();req.destroy=()=>{};
    req.end=()=>queueMicrotask(()=>{
      const r=new EventEmitter();r.statusCode=statusCode;r.headers=headers;r.destroy=()=>{};cb(r);
      r.emit('data',Buffer.from('secret fixture body'));r.emit('end');
    });return req;
  };
  for(const [h,category,media,encoding] of [
    [{'content-type':'application/json; secret=fixture','content-encoding':'gzip'},'probe_content_type','json','gzip'],
    [{'content-type':'text/event-stream','content-encoding':'br'},'probe_content_encoding','sse','br'],
    [{'content-type':'text/event-stream','content-encoding':'gzip, secret-fixture'},'probe_content_encoding','sse','other'],
    [{},'probe_content_type','missing','missing'],
    [{'content-type':'secret-fixture'},'probe_content_type','other','missing']
  ]){
    await assert.rejects(sendOnce('fixture-access',request(h)),e=>{
      assert.equal(e.message,category);assert.equal(e.metrics.http_status,200);
      assert.equal(e.metrics.content_type_class,media);assert.equal(e.metrics.content_encoding_class,encoding);
      assert.equal(e.metrics.wire_bytes,0);assert.ok(!JSON.stringify(e.metrics).includes('secret'));
      return true;
    });
  }
});
test('gzip SSE is bounded before and after decompression and feeds existing decoder',async()=>{
  const {gzipSync}=await import('node:zlib');
  const {EventEmitter}=await import('node:events');
  function fake(bytes){
    return (_url,opts,cb)=>{
      assert.equal(opts.headers['accept-encoding'],'gzip, identity');
      const req=new EventEmitter();req.destroy=()=>{};
      req.end=()=>queueMicrotask(()=>{
        const r=new EventEmitter();r.statusCode=200;r.headers={'content-type':'text/event-stream; charset=utf-8','content-encoding':'GZip'};r.destroy=()=>{};
        cb(r);for(let i=0;i<bytes.length;i+=19)r.emit('data',bytes.subarray(i,i+19));r.emit('end');
      });return req;
    };
  }
  const wire=fs.readFileSync(new URL('../../crates/atomos-core/tests/fixtures/responses-text.sse',import.meta.url));
  const zipped=gzipSync(wire);
  const r=await sendOnce('fixture-access',fake(zipped));
  assert.deepEqual(r.wire,wire);assert.equal(r.wire_bytes,zipped.length);
  assert.equal(r.decoded_bytes,wire.length);assert.equal(r.content_encoding_class,'gzip');
  assert.deepEqual(decodeWire(decoderPath(),r.wire).usage,usage);
  await assert.rejects(sendOnce('fixture-access',fake(gzipSync(Buffer.alloc(256*1024+1)))),/probe_decoded_limit/);
  await assert.rejects(sendOnce('fixture-access',fake(Buffer.from('not gzip'))),/probe_invalid_compression/);
  await assert.rejects(sendOnce('fixture-access',fake(Buffer.alloc(256*1024+1))),/probe_wire_limit/);
});
test('rejected media diagnostics settle unknown and persist without raw response',async t=>{
  const {EventEmitter}=await import('node:events');
  const dir=fixture(t);
  const request=(_url,_opts,cb)=>{
    const req=new EventEmitter();req.destroy=()=>{};
    req.end=()=>queueMicrotask(()=>{
      const r=new EventEmitter();r.statusCode=200;r.headers={'content-type':'text/html; secret=fixture'};r.destroy=()=>{};cb(r);
      r.emit('data',Buffer.from('secret body'));r.emit('end');
    });return req;
  };
  await withStore(dir,pass,async s=>{
    s.save(profile(s.host));
    const r=await qualify(s,available,{now:()=>now,send:access=>sendOnce(access,request),decode:()=>assert.fail('must not decode')});
    assert.equal(r.state,'unknown');assert.equal(r.charged_planning_units,1024);assert.equal(r.http_status,200);
    assert.equal(r.content_type_class,'html');assert.equal(r.content_encoding_class,'missing');
  });
  const raw=fs.readFileSync(path.join(dir,'m2-4-probe.json'),'utf8');assert.ok(!raw.includes('secret')&&!raw.includes('fixture-access'));
  await withStore(dir,pass,async s=>assert.rejects(qualify(s,available,{now:()=>now,send:async()=>assert.fail('must not dispatch'),decode:()=>decoded}),/probe_already_attempted/));
});


// Exact historical failure shape from the original combined header gate.
// No actual owner data is used in these fixtures.
function historical(s,overrides={}){
  const p=intent(s.profile,now);
  s.reserveProbe(p);
  const r={...resultRecord(p,null,{}),category:'probe_content_type',...overrides};
  s.finishProbe(p,r);
  return r;
}
test('separately authorized follow-up reserves before send and preserves original bytes',async t=>{
  const dir=fixture(t);let calls=0;
  const firstPath=path.join(dir,'m2-4-probe.json');
  let original;
  await withStore(dir,pass,async s=>{
    s.save(profile(s.host));historical(s);original=fs.readFileSync(firstPath);
    const r=await qualifyFollowup(s,available,{now:()=>now+1000,send:async access=>{
      calls++;assert.equal(access,'fixture-access');
      const p=JSON.parse(fs.readFileSync(path.join(dir,'m2-4-followup.json')));
      assert.equal(p.state,'pending');assert.equal(p.attempt,2);assert.equal(p.prior_retained_planning_units,1024);
      assert.match(p.previous_record_sha256,/^[a-f0-9]{64}$/);
      assert.deepEqual(fs.readFileSync(firstPath),original);
      return {wire:Buffer.from('fixture')};
    },decode:()=>decoded});
    assert.equal(r.outcome,'qualified');assert.equal(r.charged_planning_units,47);
    assert.equal(r.cumulative_planning_units,1071);assert.equal(r.prior_usage_unknown,true);
    assert.equal(r.previous_record_sha256,undefined);
  });
  await withStore(dir,pass,async s=>{
    const opts={now:()=>now+2000,send:async()=>assert.fail('no replay'),decode:()=>decoded};
    await assert.rejects(qualifyFollowup(s,available,opts),/probe_followup_already_attempted/);
    await assert.rejects(qualify(s,available,opts),/probe_already_attempted/);
  });
  assert.equal(calls,1);assert.deepEqual(fs.readFileSync(firstPath),original);
});
test('follow-up denies missing, pending, different failure and differently bound historical records',async t=>{
  const opts={now:()=>now+1000,send:async()=>assert.fail('no dispatch'),decode:()=>decoded};
  const cases=[
    ['missing',null,/probe_original_missing/],
    ['pending','pending',/probe_original_not_eligible/],
    ['quota',{category:'probe_http_429'},/probe_original_not_eligible/],
    ['new diagnostic',{http_status:200},/probe_original_not_eligible/],
    ['account mismatch',{model:'other'},/probe_original_not_eligible/],
    ['known',{state:'settled',reported_tokens:{total:47},unknown_usage:false},/probe_original_not_eligible/],
    ['time',{created_ms:now+2000},/probe_original_not_eligible/]
  ];
  for(const [,variant,code] of cases){
    const dir=fixture(t);
    await withStore(dir,pass,async s=>{
      s.save(profile(s.host));
      if(variant==='pending')s.reserveProbe(intent(s.profile,now));
      else if(variant)historical(s,variant);
      await assert.rejects(qualifyFollowup(s,available,opts),code);
      assert.equal(s.followupExists,false);
    });
  }
});
test('unknown follow-up retains both reservations and blocks any further attempt',async t=>{
  const dir=fixture(t);
  await withStore(dir,pass,async s=>{
    s.save(profile(s.host));historical(s);
    const r=await qualifyFollowup(s,available,{now:()=>now+1000,send:async()=>{throw Error('fixture');},decode:()=>decoded});
    assert.equal(r.state,'unknown');assert.equal(r.cumulative_planning_units,2048);
    assert.equal(r.prior_usage_unknown,true);assert.equal(s.followupExists,true);
  });
  await withStore(dir,pass,async s=>assert.rejects(qualifyFollowup(s,available,{now:()=>now+2000,send:async()=>assert.fail('no replay'),decode:()=>decoded}),/probe_followup_already_attempted/));
});
test('follow-up pending marker and original removal cannot permit another first attempt',async t=>{
  const dir=fixture(t);
  await withStore(dir,pass,async s=>{
    s.save(profile(s.host));historical(s);
    const first=followupEligible(s,now+1000);
    const {createHash}=await import('node:crypto');
    const p={...intent(s.profile,now+1000),attempt:2,prior_retained_planning_units:1024,previous_record_sha256:createHash('sha256').update(JSON.stringify(first)).digest('hex')};
    s.reserveFollowup(p,first);
  });
  await withStore(dir,pass,async s=>{
    const opts={now:()=>now+2000,send:async()=>assert.fail('no dispatch'),decode:()=>decoded};
    await assert.rejects(qualifyFollowup(s,available,opts),/probe_followup_already_attempted/);
  });
  fs.unlinkSync(path.join(dir,'m2-4-probe.json')); // synthetic tamper
  await withStore(dir,pass,async s=>assert.rejects(qualify(s,available,{now:()=>now+2000,send:async()=>assert.fail('no dispatch'),decode:()=>decoded}),/probe_already_attempted/));
});
