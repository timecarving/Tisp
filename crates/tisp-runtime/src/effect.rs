/// Effect runtime — evidence vectors and handler dispatch
/// Implements the Koka-style evidence passing compilation target

use std::sync::Arc;

/// An effect label identifying an effect type at runtime
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectTag(pub String);

/// Evidence for a handler — contains the handler function and its captured state
#[derive(Clone)]
pub struct Evidence {
    pub tag: EffectTag,
    /// Handler function: (operation_name, args) → result
    pub handler: Arc<dyn Fn(&str, &[Value]) -> Option<Value> + Send + Sync>,
    /// Captured state for this handler
    pub state: Option<Value>,
}

/// An evidence vector — the runtime representation of an effect row
#[derive(Clone)]
pub struct EvidenceVector {
    entries: Vec<Evidence>,
}

/// Runtime value for effect operations
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Unit,
    HandlerState(u64),
}

impl EvidenceVector {
    pub fn new() -> Self { Self { entries: Vec::new() } }

    /// Add a handler to the evidence vector
    pub fn push_handler(&mut self, evidence: Evidence) {
        self.entries.push(evidence);
    }

    /// Pop the top handler from the evidence vector
    pub fn pop_handler(&mut self) -> Option<Evidence> {
        self.entries.pop()
    }

    /// Perform an operation: find the nearest handler for the given effect tag
    pub fn perform(&self, tag: &EffectTag, op_name: &str, args: &[Value]) -> Option<Value> {
        for evidence in self.entries.iter().rev() {
            if evidence.tag == *tag {
                return (evidence.handler)(op_name, args);
            }
        }
        None
    }

    /// Check if any handler exists for a given tag
    pub fn has_handler(&self, tag: &EffectTag) -> bool {
        self.entries.iter().any(|e| e.tag == *tag)
    }

    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
}

/// Effect runtime state — thread-local evidence vector stack
pub struct EffectRuntime {
    /// Current evidence vector (the active effect context)
    pub evidence: EvidenceVector,
    /// Is the current computation yielding?
    pub is_yielding: bool,
    /// Yield continuation
    pub yield_continuation: Option<Box<dyn FnOnce() + Send>>,
}

impl EffectRuntime {
    pub fn new() -> Self {
        Self {
            evidence: EvidenceVector::new(),
            is_yielding: false,
            yield_continuation: None,
        }
    }

    /// Handle an effect operation
    pub fn handle_operation(&mut self, effect_tag: &str, op_name: &str, args: &[Value]) -> Option<Value> {
        let tag = EffectTag(effect_tag.to_string());
        self.evidence.perform(&tag, op_name, args)
    }

    /// Check if any effect handler is registered
    pub fn is_in_handler(&self) -> bool {
        !self.evidence.is_empty()
    }

    /// Set yielding state
    pub fn yield_to_handler(&mut self, continuation: Box<dyn FnOnce() + Send>) {
        self.is_yielding = true;
        self.yield_continuation = Some(continuation);
    }

    /// Resume after yield
    pub fn resume(&mut self) -> bool {
        if self.is_yielding {
            self.is_yielding = false;
            if let Some(cont) = self.yield_continuation.take() {
                cont();
                return true;
            }
        }
        false
    }
}

impl Default for EvidenceVector {
    fn default() -> Self { Self::new() }
}

impl std::fmt::Debug for EvidenceVector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EvidenceVector({} handlers)", self.entries.len())
    }
}

impl std::fmt::Debug for Evidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Evidence({:?})", self.tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_vector_push_pop() {
        let mut ev = EvidenceVector::new();
        let tag = EffectTag("State".into());
        let handler: Arc<dyn Fn(&str, &[Value]) -> Option<Value> + Send + Sync> =
            Arc::new(|op, _args| match op {
                "get" => Some(Value::Int(42)),
                "put" => Some(Value::Unit),
                _ => None,
            });
        ev.push_handler(Evidence { tag: tag.clone(), handler, state: None });
        assert!(ev.has_handler(&tag));

        let result = ev.perform(&tag, "get", &[]);
        assert_eq!(result, Some(Value::Int(42)));

        ev.pop_handler();
        assert!(!ev.has_handler(&tag));
    }

    #[test]
    fn test_effect_runtime() {
        let mut rt = EffectRuntime::new();
        assert!(!rt.is_in_handler());
        assert_eq!(rt.handle_operation("State", "get", &[]), None);
    }
}
