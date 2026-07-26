/// Persistent data structure wrappers for Tisp runtime
/// Uses im-rs under the hood for Clojure-style immutable collections

use im::{Vector, HashMap, HashSet};

/// A persistent value in the Tisp runtime
#[derive(Debug, Clone)]
pub enum PersistentValue {
    /// 64-bit signed integer
    Int(i64),
    /// 64-bit float
    Float(f64),
    /// Boolean
    Bool(bool),
    /// UTF-8 string (persistent via Arc)
    String(String),
    /// Character
    Char(char),
    /// Unit
    Unit,
    /// Persistent list (singly-linked, structural sharing)
    List(Vec<PersistentValue>),
    /// Persistent vector (HAMT, O(log32 n))
    Vec(Vector<PersistentValue>),
    /// Persistent map (HAMT)
    Map(HashMap<PersistentValue, PersistentValue>),
    /// Persistent set (HAMT)
    Set(HashSet<PersistentValue>),
    /// Keyword (interned)
    Keyword(String),
}

impl PersistentValue {
    pub fn nil() -> Self { PersistentValue::List(vec![]) }

    pub fn is_truthy(&self) -> bool {
        match self {
            PersistentValue::Bool(false) | PersistentValue::Unit => false,
            PersistentValue::Int(0) => false,
            _ => true,
        }
    }

    pub fn type_name(&self) -> &str {
        match self {
            PersistentValue::Int(_) => "i64",
            PersistentValue::Float(_) => "f64",
            PersistentValue::Bool(_) => "bool",
            PersistentValue::String(_) => "String",
            PersistentValue::Char(_) => "char",
            PersistentValue::Unit => "Unit",
            PersistentValue::List(_) => "List",
            PersistentValue::Vec(_) => "Vec",
            PersistentValue::Map(_) => "Map",
            PersistentValue::Set(_) => "Set",
            PersistentValue::Keyword(_) => "Keyword",
        }
    }
}

impl std::hash::Hash for PersistentValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            PersistentValue::Int(n) => n.hash(state),
            PersistentValue::Float(f) => f.to_bits().hash(state),
            PersistentValue::Bool(b) => b.hash(state),
            PersistentValue::String(s) => s.hash(state),
            PersistentValue::Keyword(s) => s.hash(state),
            _ => {} // Structural comparison for collections
        }
    }
}

impl PartialEq for PersistentValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (PersistentValue::Int(a), PersistentValue::Int(b)) => a == b,
            (PersistentValue::Float(a), PersistentValue::Float(b)) => a.to_bits() == b.to_bits(),
            (PersistentValue::Bool(a), PersistentValue::Bool(b)) => a == b,
            (PersistentValue::String(a), PersistentValue::String(b)) => a == b,
            (PersistentValue::Char(a), PersistentValue::Char(b)) => a == b,
            (PersistentValue::Unit, PersistentValue::Unit) => true,
            (PersistentValue::Vec(a), PersistentValue::Vec(b)) => a == b,
            (PersistentValue::Map(a), PersistentValue::Map(b)) => a == b,
            (PersistentValue::Set(a), PersistentValue::Set(b)) => a == b,
            (PersistentValue::Keyword(a), PersistentValue::Keyword(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for PersistentValue {}
