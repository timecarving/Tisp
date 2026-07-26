/// Concurrent Logic Programming runtime
/// AND-parallel, OR-parallel, Guarded Horn Clauses, committed choice

use std::sync::{Arc, Mutex};
use std::thread;
use crate::logic::ConstraintStore;
#[cfg(test)]
use crate::logic::LogicValue;

/// Guarded Horn Clause: (guard → body)
#[derive(Clone)]
pub struct GuardedClause {
    pub guard: Arc<dyn Fn(&mut ConstraintStore) -> bool + Send + Sync>,
    pub body: Arc<dyn Fn(&mut ConstraintStore) -> bool + Send + Sync>,
}

/// Concurrent logic engine with AND/OR parallelism
pub struct ConcurrentEngine {
    clauses: Vec<GuardedClause>,
    max_threads: usize,
}

type ParGoal = Arc<dyn Fn(&mut ConstraintStore) -> bool + Send + Sync>;

impl ConcurrentEngine {
    pub fn new(max_threads: usize) -> Self {
        Self { clauses: Vec::new(), max_threads: max_threads.max(1) }
    }

    pub fn add_clause(&mut self, guard: impl Fn(&mut ConstraintStore) -> bool + Send + Sync + 'static,
                      body: impl Fn(&mut ConstraintStore) -> bool + Send + Sync + 'static) {
        self.clauses.push(GuardedClause { guard: Arc::new(guard), body: Arc::new(body) });
    }

    pub fn commit_choice(&self, store: &mut ConstraintStore) -> bool {
        let depth = store.trail_depth();
        for clause in &self.clauses {
            if (clause.guard)(store) {
                let result = (clause.body)(store);
                store.cut();
                return result;
            }
            store.restore_to(depth);
        }
        false
    }

    pub fn and_parallel(&self, goals: &[ParGoal], store: &Arc<Mutex<ConstraintStore>>) -> bool {
        if goals.is_empty() { return true; }
        let results = Arc::new(Mutex::new(Vec::new()));
        let mut handles = Vec::new();
        for goal in goals.iter().take(self.max_threads) {
            let g = goal.clone(); let _s = store.clone(); let r = results.clone();
            handles.push(thread::spawn(move || {
                let mut local = ConstraintStore::new();
                r.lock().unwrap().push((g(&mut local), local));
            }));
        }
        for h in handles { h.join().ok(); }
        let all_results = results.lock().unwrap();
        all_results.iter().all(|(ok, _)| *ok)
    }

    pub fn or_parallel(&self, alternatives: &[ParGoal], store: &Arc<Mutex<ConstraintStore>>) -> Option<ConstraintStore> {
        if alternatives.is_empty() { return None; }
        let result: Arc<Mutex<Option<ConstraintStore>>> = Arc::new(Mutex::new(None));
        let mut handles = Vec::new();
        for alt in alternatives.iter().take(self.max_threads) {
            let a = alt.clone(); let s = store.clone(); let r = result.clone();
            handles.push(thread::spawn(move || {
                let mut local = s.lock().unwrap().clone();
                if a(&mut local) { let mut res = r.lock().unwrap(); if res.is_none() { *res = Some(local); } }
            }));
        }
        for h in handles { h.join().ok(); }
        let guard = result.lock().unwrap();
        guard.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_commit_choice() {
        let mut engine = ConcurrentEngine::new(4); let mut store = ConstraintStore::new(); let x = store.fresh_var();
        let x1 = x.clone(); engine.add_clause(move |s| s.unify(&x1, &LogicValue::Int(42)), move |_| true);
        let x2 = x.clone(); engine.add_clause(move |s| s.unify(&x2, &LogicValue::Int(99)), move |_| true);
        assert!(engine.commit_choice(&mut store));
    }
    #[test] fn test_or_parallel() {
        let engine = ConcurrentEngine::new(4); let store = Arc::new(Mutex::new(ConstraintStore::new()));
        let a1: ParGoal = Arc::new(|_| true); let a2: ParGoal = Arc::new(|_| false);
        assert!(engine.or_parallel(&[a1, a2], &store).is_some());
    }
}
