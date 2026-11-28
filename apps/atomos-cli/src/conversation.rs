//! Local read-only recall. Explicit grammar; no automatic learning or model routing.
use atomos_core::{respond, MockInference};
use atomos_memory::{MemoryReader, Value, ContextRequest, compile_ranked_context};

pub struct Conversation {
    reader: Option<Box<dyn MemoryReader>>,
    last_subject: Option<String>,
    local_recalls: usize,
}

const NO_FACT: &str = "I do not have that fact in my memory.";
const UNAVAILABLE: &str = "My local memory is unavailable.";

fn key(s:&str)->bool {
    !s.is_empty() && s.len()<=64 && s.bytes().all(|b|b.is_ascii_lowercase()||b.is_ascii_digit()||matches!(b,b'_'|b'-'|b'.'))
}
fn predicate(words:&str)->Option<String> {
    // No fuzzy matching. Convert ordinary space-separated words to a key.
    let p=words.split_whitespace().collect::<Vec<_>>().join("_");
    key(&p).then_some(p)
}
fn safe_text(s:&str,max:usize)->bool {
    !s.trim().is_empty() && s.len()<=max && !s.chars().any(char::is_control)
}
fn display_value(value:Value)->Option<String> {
    match value {
        Value::Text(v) if safe_text(&v,1024)=>Some(v),
        Value::Entity(v) if key(&v)=>Some(v.replace('_'," ")),
        Value::Integer(v)=>Some(v.to_string()),
        Value::Boolean(v)=>Some(if v {"yes"} else {"no"}.into()),
        _=>None,
    }
}
impl Conversation {
    pub fn new(reader:Option<Box<dyn MemoryReader>>)->Self {
        Self {reader,last_subject:None,local_recalls:0}
    }
    pub fn clear(&mut self) { self.last_subject=None; }
    pub fn local_recalls(&self)->usize { self.local_recalls }
    /// New session adapter only; preserve the legacy reply grammar and paths.
    pub fn session_local(&mut self, input: &str) -> Option<String> {
        let normalized=input.trim().trim_end_matches('?').trim().to_ascii_lowercase();
        let s=normalized.as_str();
        if ["remember","forget","correct","delete","learn"].iter().any(|w|s==*w||s.strip_prefix(*w).is_some_and(|t|t.starts_with(' '))) {
            self.clear();
            return Some("Use atomos-memory to change stored facts.".into());
        }
        let name=matches!(s,"what is your name"|"what's your name");
        let memory=s=="context"||s.starts_with("context ")||s=="recall"||s.starts_with("recall ")
            ||s.starts_with("what is my ")||s.starts_with("what's my ")||s.starts_with("and ");
        if !name&&!memory {self.clear();return None;}
        if self.reader.is_none() {
            self.clear();
            return Some(if name {"I am AtomOS."}else{"My local memory is unavailable."}.into());
        }
        if s=="recall" {self.clear();return Some("Please specify the subject and fact to recall.".into());}
        Some(self.reply(&mut MockInference::default(),input))
    }
    pub fn reply(&mut self,provider:&mut MockInference,input:&str)->String {
        let normalized=input.trim().trim_end_matches('?').trim().to_ascii_lowercase();
        let s=normalized.as_str();
        if matches!(s,"what is your name"|"what's your name") {
            self.clear();
            if let Some(reader)=&self.reader {
                let name=&reader.identity().display_name;
                return if safe_text(name,128) {format!("I am {name}.")} else {UNAVAILABLE.into()};
            }
            return respond(provider,input);
        }
        if self.reader.is_none() {
            self.clear();
            return respond(provider,input);
        }
        // Explicitly reject mutation intents in conversation; they are not tools.
        if ["remember","forget","correct","delete","learn"].iter().any(|w| s==*w||s.strip_prefix(*w).is_some_and(|tail|tail.starts_with(' '))) {
            self.clear();
            return "Use the local memory administration interface to change stored facts.".into();
        }
        // Broad retrieval and context previews are explicitly local-text only.
        // Do not normalize the actual question to lower case before validation.
        if s=="context" || s.starts_with("context ") {
            self.clear();
            let Some((anchor,question))=input.trim().split_once(' ').and_then(|(_,rest)|rest.trim().split_once(' ')) else {
                return "Use context SUBJECT QUESTION in the local text interface.".into();
            };
            let anchor=anchor.to_ascii_lowercase();
            let request=ContextRequest::new(&anchor,question.trim());
            let reader=self.reader.as_ref().expect("reader checked");
            return match reader.retrieve(&request) {
                Ok(r) if r.facts.is_empty() && !r.incomplete && r.missing_required.is_empty()=>NO_FACT.into(),
                Ok(r)=>match compile_ranked_context(&r,4096) {
                    Ok(packet)=>format!("Local context preview ({} facts, {} bytes; no inference):\n{}",packet.facts,packet.bytes,packet.data.trim_end()),
                    Err(_)=>"Context is incomplete or exceeds its budget. Narrow the question or use exact recall.".into(),
                },
                Err(_)=>"Unable to retrieve scoped context. Check the subject and question.".into(),
            };
        }
        let query=if let Some(rest)=s.strip_prefix("recall ") {
            let mut parts=rest.splitn(2,' ');
            match (parts.next(),parts.next()) {
                (Some(subject),Some(p)) if key(subject)=>predicate(p).map(|p|(subject.to_owned(),p)),
                _=>None,
            }
        } else if let Some(p)=s.strip_prefix("what is my ").or_else(||s.strip_prefix("what's my ")) {
            predicate(p).map(|p|("owner".into(),p))
        } else if let Some(p)=s.strip_prefix("and ") {
            self.last_subject.clone().and_then(|subject|predicate(p).map(|p|(subject,p)))
        } else {
            self.clear();
            return respond(provider,input);
        };
        let Some((subject,p))=query else {
            self.clear();
            return "Please specify the subject and fact to recall.".into();
        };
        self.clear();
        let reader=self.reader.as_ref().expect("reader checked");
        let result=reader.recall(&subject,&p);
        match result {
            Ok(Some(f)) if f.subject==subject && f.predicate==p && f.superseded_at.is_none()=>{
                let Some(value)=display_value(f.value) else {return UNAVAILABLE.into();};
                // Store only a subject identifier, never a fact value. Re-query
                // each turn to observe correction/deletion by the administrator.
                self.last_subject=Some(subject);
                self.local_recalls=self.local_recalls.saturating_add(1);
                format!("I have {} recorded as {}.",p.replace('_'," "),value)
            }
            Ok(Some(_))=>UNAVAILABLE.into(),
            Ok(None)=>NO_FACT.into(),
            Err(_)=>UNAVAILABLE.into(),
        }
    }
}
