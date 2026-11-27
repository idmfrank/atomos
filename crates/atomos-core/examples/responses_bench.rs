#![forbid(unsafe_code)]
// Build with --features responses-replay. Synthetic decoder-only measurement.
use atomos_core::{broker::Usage,responses::{decode,decode_compat,decode_extended,Terminal,ExtendedTerminal}};
use std::{io::{Read,Result},time::Instant};
const WIRE:&[u8]=include_bytes!("../tests/fixtures/responses-text.sse");
struct Chunks<'a>{data:&'a [u8],size:usize}
impl Read for Chunks<'_>{
    fn read(&mut self,b:&mut [u8])->Result<usize>{
        let n=self.data.len().min(b.len()).min(self.size);
        b[..n].copy_from_slice(&self.data[..n]);self.data=&self.data[n..];Ok(n)
    }
}
fn main(){
    for size in [1,64,4096]{
        let mut times=Vec::with_capacity(1000);
        for _ in 0..1000{
            let t=Instant::now();
            let d=decode(Chunks{data:WIRE,size},||false).unwrap();
            times.push(t.elapsed().as_secs_f64()*1000.0);
            assert_eq!(d.text,"Café 🚀\nReady.");
            assert_eq!(d.terminal,Terminal::Completed(Usage{input:35,output:12,cached_input:10,reasoning_output:4}));
        }
        times.sort_by(f64::total_cmp);
        println!("responses_replay chunk_bytes={size} wire_bytes={} samples=1000 errors=0 p50_ms={:.4} p95_ms={:.4} p99_ms={:.4} max_ms={:.4}",WIRE.len(),times[499],times[949],times[989],times[999]);
    }
    let compat=include_bytes!("../tests/fixtures/responses-compat.sse");
    for (profile,wire) in [("compat_reference",WIRE),("compat_auxiliary",compat.as_slice())]{
        for size in [1,64,4096]{
            let mut times=Vec::with_capacity(1000);
            for _ in 0..1000{
                let t=Instant::now();
                let (result,_)=decode_compat(Chunks{data:wire,size},||false);
                let d=result.unwrap();times.push(t.elapsed().as_secs_f64()*1000.0);
                assert_eq!(d.text,"Café 🚀\nReady.");
                assert_eq!(d.terminal,Terminal::Completed(Usage{input:35,output:12,cached_input:10,reasoning_output:4}));
            }
            times.sort_by(f64::total_cmp);
            println!("responses_candidate profile={profile} chunk_bytes={size} wire_bytes={} samples=1000 errors=0 p50_ms={:.4} p95_ms={:.4} p99_ms={:.4} max_ms={:.4}",wire.len(),times[499],times[949],times[989],times[999]);
        }
    }
    let reason=include_bytes!("../tests/fixtures/responses-reasoning.sse");
    let refusal=include_bytes!("../tests/fixtures/responses-refusal.sse");
    for (profile,wire,refused) in [("extended_reference",WIRE,false),("extended_reasoning",reason.as_slice(),false),("extended_refusal",refusal.as_slice(),true)]{
        for size in [1,64,4096]{
            let mut times=Vec::with_capacity(1000);
            for _ in 0..1000{
                let t=Instant::now();
                let (result,_)=decode_extended(Chunks{data:wire,size},||false);
                let d=result.unwrap();times.push(t.elapsed().as_secs_f64()*1000.0);
                let u=Usage{input:35,output:12,cached_input:10,reasoning_output:4};
                assert_eq!(d.text,if refused{""}else{"Café 🚀\nReady."});
                assert_eq!(d.terminal,if refused{ExtendedTerminal::Refused(u)}else{ExtendedTerminal::Completed(u)});
            }
            times.sort_by(f64::total_cmp);
            println!("responses_candidate profile={profile} chunk_bytes={size} wire_bytes={} samples=1000 errors=0 p50_ms={:.4} p95_ms={:.4} p99_ms={:.4} max_ms={:.4}",wire.len(),times[499],times[949],times[989],times[999]);
        }
    }
    println!("real_provider_calls=0");
}
