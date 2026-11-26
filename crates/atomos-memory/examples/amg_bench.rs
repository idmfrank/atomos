#![forbid(unsafe_code)]
// Synthetic fixture only. Seed in bulk; time production APIs, not seed SQL.
use atomos_memory::{compile_context,compile_ranked_context,init,open,open_reader,
    ContextRequest,MemoryReader,MemoryStore,Retrieval,Value};
use rusqlite::{params,Connection};
use std::collections::BTreeSet;
use std::path::Path;
use std::time::Instant;

fn node(i:usize)->String {format!("node{i:06}")}
fn marker(i:usize)->String {format!("marker{:06}",i/2)}
fn status(i:usize)->String {format!("{} validation {}",marker(i),if i%2==0 {"passed"} else {"complete"})}
fn sample(i:usize,n:usize)->usize {2*((i*7919+17)%(n/2))}
fn ms(start:Instant)->f64 {start.elapsed().as_secs_f64()*1000.0}
#[derive(Default)]
struct Stats { times:Vec<f64>, errors:usize }
impl Stats {
    fn add(&mut self,time:f64,ok:bool) {self.times.push(time);if !ok {self.errors+=1;}}
    fn report(&mut self,name:&str) {
        self.times.sort_by(f64::total_cmp);
        let n=self.times.len();
        if n==0 {println!("metric={name} samples=0 errors={}",self.errors);return;}
        let p=|v:usize|self.times[(n*v+99)/100-1];
        println!("metric={name} samples={n} errors={} p50_ms={:.4} p95_ms={:.4} p99_ms={:.4} max_ms={:.4}",
            self.errors,p(50),p(95),p(99),self.times[n-1]);
    }
}
fn seed(dir:&Path,n:usize) {
    let t=Instant::now();
    drop(init(dir,"bench","AMG Benchmark").expect("fresh synthetic store"));
    let mut c=Connection::open(dir.join("memory.db")).unwrap();
    c.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL; PRAGMA journal_mode=DELETE; PRAGMA cache_size=-2048; PRAGMA secure_delete=ON; PRAGMA max_page_count=32768;").unwrap();
    let version:String=c.query_row("SELECT sqlite_version()",[],|r|r.get(0)).unwrap();
    let tx=c.transaction().unwrap();
    {
        let mut entity=tx.prepare("INSERT INTO entity(id,owner_scope,entity_key,label,kind) VALUES(?1,'bench',?2,?2,'project')").unwrap();
        for i in 0..n {entity.execute(params![i as i64+2,node(i)]).unwrap();}
        let mut source=tx.prepare("INSERT INTO source(id,owner_scope,kind,recorded_at) VALUES(?1,'bench','owner_local',1700000000)").unwrap();
        let mut fact=tx.prepare("INSERT INTO assertion(id,owner_scope,subject_id,predicate,value_kind,text_value,target_id,source_id,recorded_at) VALUES(?1,'bench',?2,?3,?4,?5,?6,?1,1700000000)").unwrap();
        let filler="synthetic unrelated narrative ".repeat(6);
        let mut id=0i64;
        for i in 0..n {
            let subject=i as i64+2;
            for (p,value) in [("status",status(i)),("note",filler.clone())] {
                id+=1;source.execute([id]).unwrap();
                fact.execute(params![id,subject,p,"text",Some(value),None::<i64>]).unwrap();
            }
            if i%2==0 {
                id+=1;source.execute([id]).unwrap();
                fact.execute(params![id,subject,"linked","entity",None::<String>,Some(subject+1)]).unwrap();
            }
        }
    }
    tx.commit().unwrap();drop(c);
    println!("seed_entities={} active_assertions={} sqlite_version={} seed_ms={:.3}",n+1,n*2+n/2,version,ms(t));
}
fn expected(i:usize)->BTreeSet<(String,String)> {
    [(node(i),"status".into()),(node(i),"linked".into()),(node(i+1),"status".into())].into_iter().collect()
}
fn read(dir:&Path,n:usize,count:usize)->bool {
    let t=Instant::now();
    let r=open_reader(dir,"bench").unwrap();
    println!("metric=reader_open_process_cold samples=1 elapsed_ms={:.4}",ms(t));
    let i=sample(0,n);let t=Instant::now();
    let first=r.recall(&node(i),"status").ok().flatten();
    let first_ok=first.as_ref().is_some_and(|f|f.value==Value::Text(status(i)));
    println!("metric=first_exact_process_cold samples=1 errors={} elapsed_ms={:.4}",usize::from(!first_ok),ms(t));
    let mut exact=Stats::default();let mut retrieval=Stats::default();let mut context=Stats::default();let mut broad=Stats::default();
    let (mut hits,mut returned,mut wanted,mut compiled,mut context_bytes,mut full_bytes)=(0usize,0usize,0usize,0usize,0usize,0usize);
    for j in 0..count {
        let i=sample(j,n);let root=node(i);let peer=node(i+1);
        let t=Instant::now();
        let fact=r.recall(&root,"status").ok().flatten();
        let time=ms(t);
        exact.add(time,fact.is_some_and(|f|f.subject==root&&f.predicate=="status"&&f.value==Value::Text(status(i))));
        let q=ContextRequest::new(&root,&marker(i));
        let t=Instant::now();let rr=r.retrieve(&q);let retrieval_time=ms(t);
        let packet=rr.as_ref().ok().and_then(|v|compile_ranked_context(v,4096).ok());
        let total=ms(t);
        wanted+=3;
        let mut ok=false;
        if let Ok(ref result)=rr {
            let keys=result.facts.iter().map(|x|(x.fact.subject.clone(),x.fact.predicate.clone())).collect::<BTreeSet<_>>();
            let expected=expected(i);
            returned+=result.facts.len();hits+=keys.intersection(&expected).count();
            let values_ok=result.facts.iter().all(|x|match (x.fact.subject.as_str(),x.fact.predicate.as_str()) {
                (s,"status") if s==root.as_str()=>x.fact.value==Value::Text(status(i)),
                (s,"status") if s==peer.as_str()=>x.fact.value==Value::Text(status(i+1)),
                (s,"linked") if s==root.as_str()=>x.fact.value==Value::Entity(peer.clone()),
                _=>false,
            });
            ok=keys==expected&&result.facts.len()==3&&values_ok&&!result.incomplete&&result.omitted==0&&result.missing_required.is_empty();
        }
        retrieval.add(retrieval_time,ok);
        let complete=ok&&packet.is_some();
        context.add(total,complete);
        if let Some(packet)=packet.filter(|_|complete) {
            let mut facts=Vec::new();
            for (s,p) in [(&root,"status"),(&root,"linked"),(&root,"note"),(&peer,"status"),(&peer,"note")] {
                facts.push(r.recall(s,p).unwrap().unwrap());
            }
            let full=compile_context(&Retrieval{facts,truncated:false,visited:2},16384).unwrap();
            compiled+=1;context_bytes+=packet.bytes;full_bytes+=full.len();
        }
        if j<30 {
            let q=ContextRequest::new(&root,"status");
            let t=Instant::now();
            let good=r.retrieve(&q).ok().and_then(|v|compile_ranked_context(&v,4096).ok()).is_some_and(|p|p.facts==3);
            broad.add(ms(t),good);
        }
    }
    exact.report("exact_warm");retrieval.report("scoped_graph_fts_warm");context.report("context_end_to_end_warm");broad.report("common_term_context");
    println!("quality expected_facts={wanted} returned_facts={returned} matched_facts={hits} complete_packets={compiled} requests={count} precision={:.4} recall={:.4}",if returned==0{0.0}else{hits as f64/returned as f64},hits as f64/wanted as f64);
    println!("context_bytes successful_packets={compiled} ranked_total={context_bytes} same_scope_all_facts_total={full_bytes} remote_calls=0");
    first_ok&&exact.errors==0&&retrieval.errors==0&&context.errors==0
}
fn graph(dir:&Path,n:usize,count:usize)->bool {
    let db=open(dir,"bench").unwrap();let mut stats=Stats::default();
    for j in 0..count {
        let i=sample(j,n);let t=Instant::now();
        let result=db.neighbors(&node(i),2);let time=ms(t);
        let good=result.is_ok_and(|r|!r.truncated&&r.visited==2&&r.facts.len()==1&&r.facts[0].value==Value::Entity(node(i+1)));
        stats.add(time,good);
    }
    stats.report("graph_admin_warm");stats.errors==0
}
fn write(dir:&Path,n:usize,count:usize)->bool {
    let mut db=open(dir,"bench").unwrap();
    let mut add=Stats::default();let mut correct=Stats::default();let mut forget=Stats::default();let mut ok=true;
    for j in 0..count {
        let key=node(sample(j,n));let p=format!("benchwrite{j}");
        let t=Instant::now();let a=db.remember(&key,&p,Value::Text("before".into()));add.add(ms(t),a.is_ok());
        let Ok(a)=a else{ok=false;continue;};
        let t=Instant::now();let b=db.correct(&key,&p,a.revision,Value::Text("after".into()));correct.add(ms(t),b.is_ok());
        if b.is_err(){ok=false;continue;}
        let t=Instant::now();let deleted=db.forget(&key,&p);forget.add(ms(t),matches!(deleted,Ok(2)));
        if db.recall(&key,&p).unwrap().is_some(){ok=false;}
    }
    add.report("durable_remember");correct.report("durable_correct");forget.report("durable_forget");
    let identity=db.identity().clone();drop(db);
    let mut db=open(dir,"bench").unwrap();let t=Instant::now();let integrity=db.verify().is_ok();
    println!("metric=verify_after_writes samples=1 errors={} elapsed_ms={:.4}",usize::from(!integrity),ms(t));
    ok&&integrity&&db.identity()==&identity&&add.errors==0&&correct.errors==0&&forget.errors==0
}
fn main(){
    let a:Vec<String>=std::env::args().collect();
    if a.len()!=5 {eprintln!("usage: amg_bench seed|read|graph|write DIR ENTITY_COUNT SAMPLES");std::process::exit(2);}
    let dir=Path::new(&a[2]);let n:usize=a[3].parse().expect("entity count");let count:usize=a[4].parse().expect("samples");
    assert!((100..=100_000).contains(&n)&&n%2==0&&(1..=1000).contains(&count));
    let ok=match a[1].as_str(){"seed"=>{seed(dir,n);true},"read"=>read(dir,n,count),"graph"=>graph(dir,n,count),"write"=>write(dir,n,count),_=>false};
    if !ok {std::process::exit(1);}
}
