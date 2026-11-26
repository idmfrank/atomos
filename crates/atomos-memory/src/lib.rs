#![forbid(unsafe_code)]
//! Initial single-owner AMG contract. No network, credentials or automatic learning.
use std::path::Path;

mod sqlite;
mod retrieval;
pub use retrieval::{ContextRequest, RankedFact, RankedRetrieval, ContextPacket, compile_ranked_context};
pub use sqlite::{SqliteMemory, SqliteReader};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Invalid(&'static str),
    Missing,
    Conflict,
    WrongOwner,
    DisclosureDenied,
    UnsupportedSchema,
    Io(std::io::Error),
    Storage(rusqlite::Error),
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self { Self::Io(e) }
}
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self { Self::Storage(e) }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Do not disclose stored data or arbitrary database error text.
        let message = match self {
            Self::Invalid(m) => m,
            Self::Missing => "record not found",
            Self::Conflict => "record already exists or revision conflicts",
            Self::WrongOwner => "owner scope mismatch",
            Self::DisclosureDenied => "memory disclosure is not approved",
            Self::UnsupportedSchema => "unsupported or uninitialized AMG schema",
            Self::Io(_) => "local storage I/O failure",
            Self::Storage(_) => "SQLite operation failed",
        };
        f.write_str(message)
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub agent_id: String,
    pub owner_scope: String,
    pub display_name: String,
    pub created_at: i64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Text(String),
    Integer(i64),
    Boolean(bool),
    Entity(String),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    pub revision: i64,
    pub subject: String,
    pub predicate: String,
    pub value: Value,
    pub source_id: i64,
    pub recorded_at: i64,
    pub superseded_at: Option<i64>,
    pub supersedes: Option<i64>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retrieval {
    pub facts: Vec<Fact>,
    pub truncated: bool,
    pub visited: usize,
}

/// Engine-independent API. Fact revisions are recorded-time history.
/// The caller must authenticate/authorize mutations before invoking this API.
pub trait MemoryStore {
    fn identity(&self) -> &Identity;
    fn add_entity(&mut self, key: &str, label: &str, kind: &str) -> Result<()>;
    fn remember(&mut self, subject: &str, predicate: &str, value: Value) -> Result<Fact>;
    fn correct(&mut self, subject: &str, predicate: &str, expected: i64, value: Value) -> Result<Fact>;
    fn recall(&self, subject: &str, predicate: &str) -> Result<Option<Fact>>;
    fn forget(&mut self, subject: &str, predicate: &str) -> Result<usize>;
    fn history(&self, subject: &str, predicate: &str) -> Result<Retrieval>;
    fn search(&self, term: &str) -> Result<Retrieval>;
    fn neighbors(&self, subject: &str, depth: usize) -> Result<Retrieval>;
}

pub fn init(dir: &Path, owner: &str, name: &str) -> Result<SqliteMemory> {
    SqliteMemory::initialize(dir, owner, name)
}
pub fn open(dir: &Path, owner: &str) -> Result<SqliteMemory> {
    SqliteMemory::open(dir, owner)
}

/// Read-only runtime boundary. No mutation methods or exposed SQL.
pub trait MemoryReader {
    fn identity(&self) -> &Identity;
    fn recall(&self, subject: &str, predicate: &str) -> Result<Option<Fact>>;
    /// Revision-specific, expiring demo voice grants; default deny.
    fn recall_voice(&self, subject: &str, predicate: &str) -> Result<Option<Fact>>;
    /// Explicit scoped context retrieval; caller authorizes local text disclosure.
    /// No provider disclosure is authorized by this interface.
    fn retrieve(&self, request: &ContextRequest) -> Result<RankedRetrieval>;
}
pub fn open_reader(dir: &Path, owner: &str) -> Result<SqliteReader> {
    SqliteReader::open(dir, owner)
}

pub(crate) fn key(s: &str) -> Result<()> {
    if s.is_empty() || s.len() > 64 || !s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-' | b'.')) {
        return Err(Error::Invalid("expected a lowercase identifier of 1–64 bytes"));
    }
    Ok(())
}
pub(crate) fn text(s: &str, max: usize) -> Result<()> {
    if s.trim().is_empty() || s.len() > max || s.chars().any(char::is_control) {
        return Err(Error::Invalid("invalid or oversized text"));
    }
    Ok(())
}
/// Explicit, reversible data encoding for local context. No model instructions
/// or authorization are constructed from this data. A truncated result may not
/// be silently interpreted as complete.
pub fn compile_context(retrieval: &Retrieval, max_bytes: usize) -> Result<String> {
    if max_bytes == 0 || max_bytes > 16384 || retrieval.truncated {
        return Err(Error::Invalid("incomplete retrieval or invalid context budget"));
    }
    let mut out = String::from("AMG DATA ONLY; provenance=owner_local; recorded time, not independently verified.\n");
    if out.len() > max_bytes { return Err(Error::Invalid("context budget exceeded")); }
    for f in &retrieval.facts {
        key(&f.subject)?;
        key(&f.predicate)?;
        if f.superseded_at.is_some() { return Err(Error::Invalid("historical assertion in current context")); }
        let value = match &f.value {
            Value::Text(v) => {
                text(v, 1024)?;
                format!("text:{v:?}")
            }
            Value::Integer(v) => format!("integer:{v}"),
            Value::Boolean(v) => format!("boolean:{v}"),
            Value::Entity(v) => { key(v)?; format!("entity:{v}") }
        };
        let line = format!("{} {} {} [revision={}, source={}, recorded_at={}]\n",
            f.subject, f.predicate, value, f.revision, f.source_id, f.recorded_at);
        if out.len() + line.len() > max_bytes { return Err(Error::Invalid("context budget exceeded")); }
        out.push_str(&line);
    }
    Ok(out)
}
