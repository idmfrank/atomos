#![forbid(unsafe_code)]
// Offline decoder process. No provider access, secrets, raw output or files.
use atomos_core::{broker::Usage,responses::{decode,Terminal}};
use serde_json::json;
use std::io;
fn usage(u:Usage)->serde_json::Value{
    json!({"input":u.input,"output":u.output,"cached_input":u.cached_input,"reasoning_output":u.reasoning_output})
}
fn main(){
    let result=match decode(io::stdin().lock(),||false){
        Ok(d)=>match d.terminal{
            Terminal::Completed(u)=>json!({"terminal":"completed","usage":usage(u),"expected_text":d.text=="Hello, world!","text_bytes":d.text.len()}),
            Terminal::Failed(kind,u)=>json!({"terminal":"failed","category":format!("{kind:?}"),"usage":u.map(usage)}),
        },
        Err(e)=>json!({"terminal":"unvalidated","category":format!("{e:?}"),"usage":null}),
    };
    println!("{result}");
}
