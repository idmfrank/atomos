import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {withStore} from './store.mjs';
import {SCOPES,hostId} from './protocol.mjs';
import {MODEL,body,qualify,intent,resultRecord,decodeWire,decoderPath,sendOnce,qualifyFollowup,followupEligible,replayLoopbackCandidate,qualifyCompat,compatEligible} from './live-probe.mjs';
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


// Exercise Node's actual HTTP parser on a loopback server, not mocked
// IncomingMessage objects. Synthetic request only; no vault/provider access.
async function localResponse(t,headers,bytes,{status=200,truncate=false}={}){
  const {createServer}=await import('node:http');
  let calls=0,seen;
  const server=createServer({requestTimeout:3000,headersTimeout:3000},(req,res)=>{
    calls++;let data='';
    req.on('data',b=>{data+=b.toString();if(data.length>4096)req.destroy();});
    req.on('end',()=>{
      seen={method:req.method,url:req.url,authorization:req.headers.authorization,body:data};
      res.writeHead(status,{connection:'close',...headers});
      if(truncate){
        res.write(bytes);
        setImmediate(()=>res.destroy());
      }else{
        for(let i=0;i<bytes.length;i+=127)res.write(bytes.subarray(i,i+127));
        res.end();
      }
    });
  });
  server.setTimeout(3000,s=>s.destroy());
  await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve);});
  t.after(()=>new Promise(resolve=>{server.close(resolve);server.closeAllConnections();}));
  const port=server.address().port;
  return {port,seen:()=>seen,calls:()=>calls};
}
function syntheticWire(){return fs.readFileSync(new URL('../../crates/atomos-core/tests/fixtures/responses-text.sse',import.meta.url));}
test('real loopback: current strict transport rejects missing MIME; offline candidate validates the identical stream',async t=>{
  const {request}=await import('node:http');
  const wire=syntheticWire(),f=await localResponse(t,{},wire);
  const adapter=(_url,opts,cb)=>request({hostname:'127.0.0.1',port:f.port,path:'/v1/responses',method:opts.method,headers:opts.headers,agent:false},cb);
  await assert.rejects(sendOnce('fixture-access',adapter),e=>e.message==='probe_content_type'&&e.metrics.content_type_class==='missing'&&e.metrics.wire_bytes===0);
  const r=await replayLoopbackCandidate(f.port);
  assert.equal(r.metrics.http_status,200);assert.equal(r.metrics.content_type_class,'missing');assert.equal(r.metrics.content_encoding_class,'missing');
  assert.equal(r.metrics.decoded_bytes,wire.length);assert.equal(r.decoded.terminal,'completed');assert.deepEqual(r.decoded.usage,usage);
  assert.equal(r.decoded.expected_text,false);assert.equal('wire' in r,false);
  assert.deepEqual(f.seen(),{method:'POST',url:'/v1/responses',authorization:'Bearer fixture-access',body:body()});assert.equal(f.calls(),2);
});
test('real loopback: missing MIME does not authorize empty, JSON, HTML, malformed or nonterminal bodies',async t=>{
  const wire=syntheticWire();
  const terminal=wire.indexOf(Buffer.from('event: response.completed'));
  assert.ok(terminal>0);
  for(const bytes of [Buffer.alloc(0),Buffer.from('{"fixture":true}'),Buffer.from('<html>fixture</html>'),Buffer.from('event: response.created\ndata: {bad}\n\n'),wire.subarray(0,terminal),Buffer.from(wire.toString().replace('"total_tokens":47','"total_tokens":48'))]){
    const f=await localResponse(t,{},bytes);
    const r=await replayLoopbackCandidate(f.port);
    assert.equal(r.metrics.content_type_class,'missing');assert.equal(r.decoded.terminal,'unvalidated');
    assert.equal(resultRecord(intent(profile(),now),r.decoded,{}).state,'unknown');assert.equal(f.calls(),1);
  }
});
test('real loopback: explicit non-SSE media or unsupported encoding stays rejected',async t=>{
  for(const [h,code] of [[{'content-type':'application/json'},'probe_content_type'],[{'content-type':'text/html'},'probe_content_type'],[{'content-type':'text/plain'},'probe_content_type'],[{'content-type':''},'probe_content_type'],[{'content-encoding':'br'},'probe_content_encoding'],[{'content-encoding':'gzip, identity'},'probe_content_encoding']]){
    const f=await localResponse(t,h,syntheticWire());
    await assert.rejects(replayLoopbackCandidate(f.port),e=>e.message===code&&e.metrics.wire_bytes===0);
    assert.equal(f.calls(),1);
  }
});
test('real loopback: gzip, independent byte limits, invalid compression and declared truncation',async t=>{
  const {gzipSync}=await import('node:zlib');
  const wire=syntheticWire(),zipped=gzipSync(wire);
  const valid=await localResponse(t,{'content-encoding':'gzip'},zipped);
  const r=await replayLoopbackCandidate(valid.port);
  assert.equal(r.metrics.wire_bytes,zipped.length);assert.equal(r.metrics.decoded_bytes,wire.length);assert.deepEqual(r.decoded.usage,usage);
  for(const [h,bytes,code] of [[{},Buffer.alloc(256*1024+1),'probe_wire_limit'],[{'content-encoding':'gzip'},gzipSync(Buffer.alloc(256*1024+1)),'probe_decoded_limit'],[{'content-encoding':'gzip'},Buffer.from('not gzip'),'probe_invalid_compression']]){
    const f=await localResponse(t,h,bytes);
    await assert.rejects(replayLoopbackCandidate(f.port),e=>e.message===code);
  }
  const truncated=await localResponse(t,{'content-length':wire.length+100},wire,{truncate:true});
  await assert.rejects(replayLoopbackCandidate(truncated.port),/probe_stream_interrupted|probe_transport_error/);
});
test('real loopback: no redirect or alternate endpoint, no invalid port',async t=>{
  const f=await localResponse(t,{location:'http://127.0.0.1:1/'},Buffer.alloc(0),{status:302});
  await assert.rejects(replayLoopbackCandidate(f.port),/probe_http_302/);assert.equal(f.calls(),1);
  for(const port of [0,-1,65536,'1234',null])await assert.rejects(replayLoopbackCandidate(port),/invalid_fixture_port/);
});


async function twoHistorical(s,overrides={}){
  const {createHash}=await import('node:crypto');
  const first=historical(s);
  const second={...intent(s.profile,now+1000),attempt:2,prior_retained_planning_units:1024,previous_record_sha256:createHash('sha256').update(JSON.stringify(first)).digest('hex')};
  s.reserveFollowup(second,first);
  const r={...resultRecord(second,null,{}),category:'probe_content_type',http_status:200,content_type_class:'missing',content_encoding_class:'missing',first_body_ms:null,wire_bytes:0,prior_usage_unknown:true,cumulative_planning_units:2048,...overrides};
  s.finishFollowup(second,r,first);
  return {first,second:r};
}
test('compat qualification preserves both records and accounts separately before dispatch',async t=>{
  const dir=fixture(t),names=['m2-4-probe.json','m2-4-followup.json'];let originals,calls=0;
  await withStore(dir,pass,async s=>{
    s.save(profile(s.host));await twoHistorical(s);originals=names.map(n=>fs.readFileSync(path.join(dir,n)));
    const r=await qualifyCompat(s,available,{now:()=>now+2000,send:async access=>{
      assert.equal(access,'fixture-access');calls++;
      const p=JSON.parse(fs.readFileSync(path.join(dir,'m2-4-compat.json')));
      assert.equal(p.state,'pending');assert.equal(p.attempt,3);assert.equal(p.prior_retained_planning_units,2048);
      assert.match(p.original_record_sha256,/^[0-9a-f]{64}$/);assert.match(p.previous_record_sha256,/^[0-9a-f]{64}$/);
      assert.deepEqual(names.map(n=>fs.readFileSync(path.join(dir,n))),originals);
      return {wire:Buffer.from('fixture')};
    },decode:()=>({...decoded,diagnostics:{stage:'terminal',event_class:'response.completed',item_class:'none',validated_events:10,secret:'fixture secret'}})});
    assert.equal(r.outcome,'qualified');assert.equal(r.cumulative_planning_units,2095);assert.equal(r.prior_usage_unknown,true);assert.equal(r.original_record_sha256,undefined);assert.equal(r.previous_record_sha256,undefined);
    assert.deepEqual(r.decoder_diagnostics,{stage:'terminal',event_class:'response.completed',item_class:'none',validated_events:10});
  });
  await withStore(dir,pass,async s=>{
    const o={now:()=>now+3000,send:async()=>assert.fail('no dispatch'),decode:()=>decoded};
    await assert.rejects(qualifyCompat(s,available,o),/probe_compat_already_attempted/);
    await assert.rejects(qualifyFollowup(s,available,o),/probe_followup_already_attempted/);
    await assert.rejects(qualify(s,available,o),/probe_already_attempted/);
  });
  assert.equal(calls,1);assert.deepEqual(names.map(n=>fs.readFileSync(path.join(dir,n))),originals);
});
test('compat qualification refuses missing or incompatible history and expired grants',async t=>{
  const o={now:()=>now+2000,send:async()=>assert.fail('no dispatch'),decode:()=>decoded};
  const cases=[null,{category:'probe_http_429'},{content_type_class:'sse'},{wire_bytes:1},{http_status:500},{first_body_ms:1},{created_ms:now+3000}];
  for(const variant of cases){
    const dir=fixture(t);
    await withStore(dir,pass,async s=>{
      s.save(profile(s.host));
      if(variant)await twoHistorical(s,variant);else historical(s);
      await assert.rejects(qualifyCompat(s,available,o),variant?/probe_compat_history_ineligible/:/probe_compat_history_missing/);
      assert.equal(s.compatExists,false);
    });
  }
  const dir=fixture(t);
  await withStore(dir,pass,async s=>{
    s.save(profile(s.host));await twoHistorical(s);
    await assert.rejects(qualifyCompat(s,available,{...o,now:()=>now+3600000}),/catalog_requires_current_grant/);
    assert.equal(s.compatExists,false);
  });
});
test('compat unknown result retains all three reservations; rejects arbitrary decoder diagnostic labels',async t=>{
  const dir=fixture(t);
  await withStore(dir,pass,async s=>{
    s.save(profile(s.host));await twoHistorical(s);
    const r=await qualifyCompat(s,available,{now:()=>now+2000,send:async()=>({wire:Buffer.from('fixture')}),decode:()=>({terminal:'unvalidated',category:'Unsupported',diagnostics:{stage:'created',event_class:'private-fixture',item_class:'other',validated_events:1}})});
    assert.equal(r.state,'unknown');assert.equal(r.cumulative_planning_units,3072);assert.equal(r.decoder_diagnostics,undefined);
  });
  assert.ok(!fs.readFileSync(path.join(dir,'m2-4-compat.json'),'utf8').includes('private-fixture'));
  await withStore(dir,pass,async s=>assert.rejects(qualifyCompat(s,available,{now:()=>now+3000,send:async()=>assert.fail('no replay'),decode:()=>decoded}),/probe_compat_already_attempted/));
});
test('compat pending crash marker blocks all prior paths even after historical marker removal',async t=>{
  const dir=fixture(t),{createHash}=await import('node:crypto');
  await withStore(dir,pass,async s=>{
    s.save(profile(s.host));await twoHistorical(s);
    const {first,second}=compatEligible(s,now+2000);
    const digest=x=>createHash('sha256').update(JSON.stringify(x)).digest('hex');
    s.reserveCompat({...intent(s.profile,now+2000),attempt:3,prior_retained_planning_units:2048,original_record_sha256:digest(first),previous_record_sha256:digest(second)},first,second);
  });
  fs.unlinkSync(path.join(dir,'m2-4-probe.json'));fs.unlinkSync(path.join(dir,'m2-4-followup.json')); // synthetic tamper
  await withStore(dir,pass,async s=>{
    const o={now:()=>now+3000,send:async()=>assert.fail('no dispatch'),decode:()=>decoded};
    await assert.rejects(qualifyCompat(s,available,o),/probe_compat_already_attempted/);
    await assert.rejects(qualifyFollowup(s,available,o),/probe_followup_already_attempted/);
    await assert.rejects(qualify(s,available,o),/probe_already_attempted/);
  });
});
test('Rust diagnostic snapshot identifies unsupported reasoning item without exposing its contents',()=>{
  const wire=syntheticWire().toString().replace('"id":"msg_synthetic","type":"message","status":"in_progress"','"id":"msg_synthetic","type":"reasoning","status":"in_progress"');
  const r=decodeWire(decoderPath(),Buffer.from(wire));
  assert.equal(r.terminal,'unvalidated');assert.equal(r.category,'Unsupported');
  assert.deepEqual(r.diagnostics,{stage:'in_progress',event_class:'response.output_item.added',item_class:'reasoning',validated_events:2});
  assert.ok(!JSON.stringify(r).includes('msg_synthetic'));
  const valid=decodeWire(decoderPath(),syntheticWire());
  assert.deepEqual(valid.diagnostics,{stage:'terminal',event_class:'response.completed',item_class:'none',validated_events:10});
  const malformed=decodeWire(decoderPath(),Buffer.from('event: response.created\ndata: {bad}\n\n'));
  assert.equal(malformed.category,'Json');assert.equal(malformed.diagnostics.event_class,'unparsed');assert.equal(malformed.diagnostics.validated_events,0);
});
