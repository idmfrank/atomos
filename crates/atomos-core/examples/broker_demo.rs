//! Explicit synthetic replay, not a live conversation interface.
use atomos_core::{broker::{Broker,Event,Request,Usage},governor::{Limits,Priority}};
fn main(){
    let mut args=std::env::args_os().skip(1);
    let root=args.next().expect("usage: broker_demo SYNTHETIC_LEDGER_ROOT");
    assert!(args.next().is_none());
    let mut b=Broker::open(std::path::Path::new(&root),Limits::default()).expect("synthetic ledger");
    let make=|text:&'static str|Request{turn:1,task:1,text,priority:Priority::Interactive,max_output:64};
    let local=b.run(make("what is your name?"),1,&[]).unwrap();
    println!("{}",local.text);
    let answer=b.run(make("explain entropy"),1,&[
        Event::Started{turn:1},
        Event::Delta{turn:1,text:"Synthetic replay: "},
        Event::Delta{turn:1,text:"entropy example."},
        Event::Completed{turn:1,usage:Usage{input:35,output:12,cached_input:10,reasoning_output:4}},
    ]).unwrap();
    println!("{}",answer.text);
    println!("Synthetic dispatches: {}",b.dispatches());
    println!("Synthetic charged units: {}",b.charged());
    println!("Real provider calls: 0");
}
