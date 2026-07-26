use tisp_core::symbol::Symbol;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Process runtime supporting π-calculus channels
pub struct ProcessRuntime {
    channels: HashMap<Symbol, Channel>,
}

#[derive(Clone)]
struct Channel {
    buffer: Arc<Mutex<Vec<Value>>>,
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Str(String),
    Unit,
    Chan(Symbol),
}

impl ProcessRuntime {
    pub fn new() -> Self {
        Self { channels: HashMap::new() }
    }

    pub fn new_channel(&mut self, name: Symbol) -> Value {
        self.channels.insert(name.clone(), Channel { buffer: Arc::new(Mutex::new(Vec::new())) });
        Value::Chan(name)
    }

    pub fn send(&self, chan_name: &Symbol, val: Value) {
        if let Some(ch) = self.channels.get(chan_name) {
            ch.buffer.lock().unwrap().push(val);
        }
    }

    pub fn recv(&self, chan_name: &Symbol) -> Option<Value> {
        self.channels.get(chan_name).and_then(|ch| ch.buffer.lock().unwrap().pop())
    }

    pub fn has_channel(&self, name: &Symbol) -> bool {
        self.channels.contains_key(name)
    }
}

/// Simple model checker for reachability properties
pub struct ModelChecker {
    max_depth: usize,
}

#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub property_holds: bool,
    pub trace: Vec<String>,
    pub depth: usize,
}

impl ModelChecker {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }

    /// Check if a state is reachable from an initial state given transition rules
    pub fn check_reachability<T: Clone + Eq + std::hash::Hash + std::fmt::Debug>(
        &self,
        initial: T,
        target_predicate: impl Fn(&T) -> bool,
        transitions: impl Fn(&T) -> Vec<T>,
    ) -> VerificationResult {
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        let mut parent: HashMap<String, (String, usize)> = HashMap::new();

        let init_key = format!("{:?}", initial);
        queue.push_back((initial.clone(), 0));
        visited.insert(init_key.clone());
        parent.insert(init_key, ("start".to_string(), 0));

        while let Some((state, depth)) = queue.pop_front() {
            let state_key = format!("{:?}", state);

            if target_predicate(&state) {
                // Reconstruct trace
                let mut trace = Vec::new();
                let mut current = state_key;
                while let Some((parent_key, d)) = parent.get(&current).cloned() {
                    trace.push(format!("depth {}: {}", d, current));
                    if parent_key == "start" { break; }
                    current = parent_key;
                }
                trace.reverse();
                return VerificationResult { property_holds: true, trace, depth };
            }

            if depth >= self.max_depth { continue; }

            for next in transitions(&state) {
                let next_key = format!("{:?}", next);
                if !visited.contains(&next_key) {
                    visited.insert(next_key.clone());
                    parent.insert(next_key, (state_key.clone(), depth + 1));
                    queue.push_back((next, depth + 1));
                }
            }
        }

        VerificationResult { property_holds: false, trace: vec![], depth: self.max_depth }
    }
}
