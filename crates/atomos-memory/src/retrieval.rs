//! Bounded, deterministic context retrieval for explicit local text requests.
//! The output is data, not an instruction or an inference permit.
use crate::{compile_context, key, text, Error, Fact, Result, Retrieval, Value};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_TERMS: usize = 8;
pub const MAX_CANDIDATES: usize = 50;
pub const MAX_CONTEXT_FACTS: usize = 50;

#[derive(Debug, Clone)]
pub struct ContextRequest {
    pub anchor: String,
    pub question: String,
    /// Exact predicates on the anchor that must be present in the packet.
    pub required: Vec<String>,
    pub depth: usize,
    pub max_facts: usize,
}
impl ContextRequest {
    pub fn new(anchor: &str, question: &str) -> Self {
        Self { anchor: anchor.into(), question: question.into(), required: Vec::new(),
            depth: 2, max_facts: 12 }
    }
    pub(crate) fn validate(&self) -> Result<Vec<String>> {
        key(&self.anchor)?;
        text(&self.question, 512)?;
        if !(0..=3).contains(&self.depth) || !(1..=MAX_CONTEXT_FACTS).contains(&self.max_facts)
            || self.required.len() > 8 {
            return Err(Error::Invalid("invalid context request bounds"));
        }
        for p in &self.required { key(p)?; }
        if self.required.iter().collect::<BTreeSet<_>>().len() != self.required.len() {
            return Err(Error::Invalid("duplicate required predicate"));
        }
        let terms = words(&self.question);
        let unique: BTreeSet<_> = terms.into_iter().filter(|w| !stop(w)).collect();
        if unique.len() > MAX_TERMS || unique.iter().any(|w| w.len() > 64) {
            return Err(Error::Invalid("too many or oversized search terms"));
        }
        if unique.is_empty() && self.required.is_empty() {
            return Err(Error::Invalid("context requires a search term or required predicate"));
        }
        Ok(unique.into_iter().collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedFact {
    pub fact: Fact,
    pub score: u32,
    pub required: bool,
    /// A connector may be included solely to explain a graph-linked match.
    pub connector: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedRetrieval {
    pub facts: Vec<RankedFact>,
    pub missing_required: Vec<String>,
    pub visited: usize,
    pub candidates: usize,
    pub omitted: usize,
    pub incomplete: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextPacket {
    pub data: String,
    pub bytes: usize,
    pub facts: usize,
}
pub(crate) fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty()).map(str::to_lowercase).collect()
}
fn stop(s: &str) -> bool {
    matches!(s,"a"|"an"|"and"|"are"|"about"|"at"|"be"|"by"|"can"|"did"|"do"|"does"|
        "for"|"from"|"how"|"i"|"in"|"is"|"it"|"me"|"my"|"of"|"on"|"or"|"please"|
        "tell"|"that"|"the"|"to"|"was"|"what"|"when"|"where"|"which"|"who"|"with"|"you"|"your")
}
fn overlap(terms: &[String], s: &str) -> u32 {
    let w: BTreeSet<_> = words(s).into_iter().collect();
    terms.iter().filter(|t| w.contains(*t)).count() as u32
}
/// Stable, explainable lexical score. Recorded time is a tie-breaker only,
/// not a truth or confidence measure. All v1 sources are owner_local.
pub(crate) fn score(f: &Fact, terms: &[String], distance: usize, fts_position: usize) -> u32 {
    let value = match &f.value {
        Value::Text(s)|Value::Entity(s) => s.clone(),
        Value::Integer(i) => i.to_string(),
        Value::Boolean(b) => b.to_string(),
    };
    100 + 24*overlap(terms,&f.predicate) + 12*overlap(terms,&value)
        + 6*overlap(terms,&f.subject) + (3-distance.min(3)) as u32*8
        + (MAX_CANDIDATES.saturating_sub(fts_position)/10) as u32
}
/// Each candidate carries its path from the anchor. Select whole bundles,
/// never a fact stripped of its graph evidence. Required exact facts first.
pub(crate) fn rank(
    mut matches: Vec<(Fact, u32, Vec<Fact>)>,
    required: Vec<Fact>, missing_required: Vec<String>,
    visited: usize, incomplete: bool, max: usize,
) -> RankedRetrieval {
    matches.sort_by(|a,b| b.1.cmp(&a.1)
        .then_with(|| b.0.recorded_at.cmp(&a.0.recorded_at))
        .then_with(|| a.0.revision.cmp(&b.0.revision)));
    let candidates=matches.len();
    let mut selected: Vec<RankedFact>=Vec::new();
    let mut ids=BTreeSet::new();
    let mut omitted=0;
    let mut incomplete=incomplete;
    for fact in required {
        if selected.len()>=max { incomplete=true; omitted+=1; continue; }
        ids.insert(fact.revision);
        selected.push(RankedFact {fact,score:u32::MAX,required:true,connector:false});
    }
    for (fact,score,path) in matches {
        if ids.contains(&fact.revision) {
            // If a connector also matched FTS, promote its relevance without
            // duplicating the fact or changing required status.
            if let Some(old)=selected.iter_mut().find(|x|x.fact.revision==fact.revision) {
                if !old.required { old.connector=false; old.score=score; }
            }
            continue;
        }
        let mut bundle=BTreeMap::new();
        for edge in path { if !ids.contains(&edge.revision) { bundle.insert(edge.revision,edge); } }
        bundle.remove(&fact.revision);
        if selected.len()+bundle.len()+1>max { omitted+=1; continue; }
        for (_,edge) in bundle {
            ids.insert(edge.revision);
            selected.push(RankedFact {fact:edge,score:0,required:false,connector:true});
        }
        ids.insert(fact.revision);
        selected.push(RankedFact {fact,score,required:false,connector:false});
    }
    RankedRetrieval { facts:selected,missing_required,visited,candidates,omitted,incomplete }
}

/// Conservative first compiler: reject work truncation, missing mandatory facts,
/// ranked omissions and byte overflow. No partial packet or inferred facts.
/// The caller must keep data separate from trusted policy and user instructions.
/// Bytes are not provider tokens; a future broker must estimate full usage.
pub fn compile_ranked_context(r: &RankedRetrieval, max_bytes: usize) -> Result<ContextPacket> {
    if r.incomplete || r.omitted>0 || !r.missing_required.is_empty() || r.facts.is_empty() {
        return Err(Error::Invalid("incomplete, empty or over-budget context retrieval"));
    }
    let facts=r.facts.iter().map(|x|x.fact.clone()).collect::<Vec<_>>();
    let n=facts.len();
    let data=compile_context(&Retrieval {facts,truncated:false,visited:r.visited},max_bytes)?;
    Ok(ContextPacket {bytes:data.len(),facts:n,data})
}
