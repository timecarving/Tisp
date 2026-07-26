/// Mercury-style logic programming runtime
/// Unification, backtracking trail, and search strategies

use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

/// A logic variable — may be bound or unbound
#[derive(Debug, Clone)]
pub enum LVar {
    /// Unbound variable with a unique ID
    Free(u64),
    /// Bound to a concrete value
    Bound(Box<LogicValue>),
}

/// Values in the logic programming domain
#[derive(Debug, Clone, PartialEq)]
pub enum LogicValue {
    Int(i64),
    Str(String),
    Bool(bool),
    Cons(Box<LogicValue>, Box<LogicValue>),
    Nil,
    Var(u64), // Reference to another LVar
}

/// The unification trail — records bindings for backtracking
#[derive(Debug, Clone)]
struct TrailEntry {
    var_id: u64,
    old_value: Option<LVar>,
}

/// The constraint store — holds all logic variables and their bindings
#[derive(Debug, Clone)]
pub struct ConstraintStore {
    vars: HashMap<u64, LVar>,
    next_id: u64,
    trail: Vec<TrailEntry>,
    /// Choice point: (trail_depth_before, alternative_branches_left)
    choice_points: Vec<(usize, usize)>,
}

impl ConstraintStore {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            next_id: 0,
            trail: Vec::new(),
            choice_points: Vec::new(),
        }
    }

    /// Create a fresh logic variable
    pub fn fresh_var(&mut self) -> LogicValue {
        let id = self.next_id;
        self.next_id += 1;
        self.vars.insert(id, LVar::Free(id));
        LogicValue::Var(id)
    }

    /// Create n fresh logic variables
    pub fn fresh_vars(&mut self, n: usize) -> Vec<LogicValue> {
        (0..n).map(|_| self.fresh_var()).collect()
    }

    /// Dereference a logic variable to its final value (chase chain)
    pub fn deref(&self, val: &LogicValue) -> LogicValue {
        match val {
            LogicValue::Var(id) => {
                if let Some(LVar::Bound(inner)) = self.vars.get(id) {
                    let deref_inner = self.deref(inner);
                    if deref_inner != **inner {
                        return deref_inner;
                    }
                    (**inner).clone()
                } else {
                    val.clone()
                }
            }
            _ => val.clone(),
        }
    }

    /// Unify two logic values — returns true if successful
    pub fn unify(&mut self, a: &LogicValue, b: &LogicValue) -> bool {
        let a_deref = self.deref(a);
        let b_deref = self.deref(b);

        match (&a_deref, &b_deref) {
            (LogicValue::Var(a_id), LogicValue::Var(b_id)) if a_id == b_id => true,

            (LogicValue::Var(_id), other) | (other, LogicValue::Var(_id)) => {
                // Bind the variable
                let id = match (&a_deref, &b_deref) {
                    (LogicValue::Var(id), _) | (_, LogicValue::Var(id)) => *id,
                    _ => unreachable!(),
                };
                // Occurs check
                if let LogicValue::Var(v_id) = other {
                    if *v_id == id {
                        // Self-reference check handled above
                    }
                }
                self.trail.push(TrailEntry {
                    var_id: id,
                    old_value: self.vars.get(&id).cloned(),
                });
                self.vars.insert(id, LVar::Bound(Box::new(other.clone())));
                true
            }

            (LogicValue::Int(a), LogicValue::Int(b)) => a == b,
            (LogicValue::Str(a), LogicValue::Str(b)) => a == b,
            (LogicValue::Bool(a), LogicValue::Bool(b)) => a == b,
            (LogicValue::Nil, LogicValue::Nil) => true,

            (LogicValue::Cons(a_h, a_t), LogicValue::Cons(b_h, b_t)) => {
                self.unify(a_h, b_h) && self.unify(a_t, b_t)
            }

            _ => false,
        }
    }

    /// Mark a choice point (before trying alternatives)
    pub fn mark_choice_point(&mut self) {
        self.choice_points.push((self.trail.len(), 0));
    }

    /// Cut — commit to current choice, discard alternatives
    pub fn cut(&mut self) {
        if let Some((_trail_depth, _)) = self.choice_points.pop() {
            // Keep bindings after the choice point
            // Discard the choice point itself
        }
    }

    /// Backtrack — undo bindings to the last choice point
    pub fn backtrack(&mut self) -> bool {
        if let Some((trail_depth, _)) = self.choice_points.last() {
            let depth = *trail_depth;
            while self.trail.len() > depth {
                let entry = self.trail.pop().unwrap();
                if let Some(old) = entry.old_value {
                    self.vars.insert(entry.var_id, old);
                } else {
                    self.vars.remove(&entry.var_id);
                }
            }
            true
        } else {
            false
        }
    }

    /// Get the trail depth (for saving/restoring state)
    pub fn trail_depth(&self) -> usize {
        self.trail.len()
    }

    /// Restore to a specific trail depth
    pub fn restore_to(&mut self, depth: usize) {
        while self.trail.len() > depth {
            let entry = self.trail.pop().unwrap();
            if let Some(old) = entry.old_value {
                self.vars.insert(entry.var_id, old);
            } else {
                self.vars.remove(&entry.var_id);
            }
        }
    }

    /// Extract a concrete value from a logic value
    pub fn extract(&self, val: &LogicValue) -> Option<LogicValue> {
        let derefed = self.deref(val);
        match &derefed {
            LogicValue::Var(_) => None, // Still unbound
            LogicValue::Cons(h, t) => {
                let h_val = self.extract(h)?;
                let t_val = self.extract(t)?;
                Some(LogicValue::Cons(Box::new(h_val), Box::new(t_val)))
            }
            other => Some(other.clone()),
        }
    }
}

/// A goal is a function that takes a state and produces a stream of results
pub type Goal = Rc<dyn Fn(&mut ConstraintStore) -> bool>;

/// Create a goal that always succeeds
pub fn succeed() -> Goal {
    Rc::new(|_| true)
}

/// Create a goal that always fails
pub fn fail() -> Goal {
    Rc::new(|_| false)
}

/// Conjunction of two goals (AND)
pub fn conj(g1: Goal, g2: Goal) -> Goal {
    Rc::new(move |store| {
        let depth = store.trail_depth();
        store.mark_choice_point();
        if g1(store) {
            let result = g2(store);
            if !result {
                store.restore_to(depth);
            }
            result
        } else {
            store.restore_to(depth);
            false
        }
    })
}

/// Disjunction of two goals (OR)
pub fn disj(g1: Goal, g2: Goal) -> Goal {
    Rc::new(move |store| {
        let depth = store.trail_depth();
        if g1(store) {
            true
        } else {
            store.restore_to(depth);
            g2(store)
        }
    })
}

/// Unification goal
pub fn eq(a: LogicValue, b: LogicValue) -> Goal {
    Rc::new(move |store| store.unify(&a, &b))
}

/// Committed choice — try the first goal that succeeds and commit
pub fn commit(goals: Vec<Goal>) -> Goal {
    Rc::new(move |store| {
        let depth = store.trail_depth();
        for g in &goals {
            if g(store) {
                store.cut();
                return true;
            }
            store.restore_to(depth);
        }
        false
    })
}

/// DFS search engine — find all solutions
pub struct SearchEngine {
    max_depth: usize,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub bindings: Vec<(u64, LogicValue)>,
    pub depth: usize,
}

impl SearchEngine {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }

    /// Find all solutions using DFS
    pub fn find_all(&self, goal: &Goal, vars: &[LogicValue]) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let mut stack: Vec<(ConstraintStore, usize)> = Vec::new();
        stack.push((ConstraintStore::new(), 0));

        while let Some((store, depth)) = stack.pop() {
            if depth >= self.max_depth {
                continue;
            }

            let mut store = store.clone();
            if goal(&mut store) {
                // Extract bindings for the requested variables
                let mut bindings = Vec::new();
                for v in vars {
                    if let LogicValue::Var(id) = v {
                        if let Some(LVar::Bound(val)) = store.vars.get(id) {
                            bindings.push((*id, (**val).clone()));
                        }
                    }
                }
                results.push(SearchResult { bindings: bindings.clone(), depth });

                // For multiple solutions: backtrack and try alternatives
                if !store.backtrack() {
                    continue; // No more alternatives
                }
                // Re-push for more solutions at same depth
                stack.push((store, depth + 1));
            }
        }

        results
    }

    /// Find first solution using DFS
    pub fn find_first(&self, goal: &Goal) -> Option<SearchResult> {
        let mut store = ConstraintStore::new();
        if goal(&mut store) {
            let mut bindings = Vec::new();
            for (id, lvar) in &store.vars {
                if let LVar::Bound(val) = lvar {
                    bindings.push((*id, (**val).clone()));
                }
            }
            Some(SearchResult { bindings, depth: 0 })
        } else {
            None
        }
    }
}

/// BFS search engine — find all solutions level by level
pub struct BfsSearchEngine {
    max_depth: usize,
}

impl BfsSearchEngine {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }

    pub fn find_all(&self, goal: &Goal, vars: &[LogicValue]) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back((ConstraintStore::new(), 0));

        while let Some((store, depth)) = queue.pop_front() {
            if depth >= self.max_depth {
                continue;
            }

            let mut store = store.clone();
            if goal(&mut store) {
                let mut bindings = Vec::new();
                for v in vars {
                    if let LogicValue::Var(id) = v {
                        if let Some(LVar::Bound(val)) = store.vars.get(id) {
                            bindings.push((*id, (**val).clone()));
                        }
                    }
                }
                results.push(SearchResult { bindings, depth: depth + 1 });
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fresh_var() {
        let mut store = ConstraintStore::new();
        let x = store.fresh_var();
        assert!(matches!(x, LogicValue::Var(_)));
    }

    #[test]
    fn test_unify_int() {
        let mut store = ConstraintStore::new();
        assert!(store.unify(&LogicValue::Int(42), &LogicValue::Int(42)));
        assert!(!store.unify(&LogicValue::Int(42), &LogicValue::Int(43)));
    }

    #[test]
    fn test_unify_var() {
        let mut store = ConstraintStore::new();
        let x = store.fresh_var();
        assert!(store.unify(&x, &LogicValue::Int(42)));

        // Variable should now be bound
        let derefed = store.deref(&x);
        assert_eq!(derefed, LogicValue::Int(42));
    }

    #[test]
    fn test_backtrack() {
        let mut store = ConstraintStore::new();
        let x = store.fresh_var();

        let depth = store.trail_depth();
        store.unify(&x, &LogicValue::Int(42));
        assert!(store.deref(&x) == LogicValue::Int(42));

        store.restore_to(depth);
        // After restore, x should be unbound again
        assert!(matches!(store.deref(&x), LogicValue::Var(_)));
    }

    #[test]
    fn test_conj_disj() {
        let mut store = ConstraintStore::new();
        let x = store.fresh_var();

        let g1 = eq(x.clone(), LogicValue::Int(42));
        let g2 = eq(x.clone(), LogicValue::Int(42));
        assert!(conj(g1, g2)(&mut store));

        let mut store = ConstraintStore::new();
        let x = store.fresh_var();
        let g1 = eq(x.clone(), LogicValue::Int(42));
        let g2 = eq(x.clone(), LogicValue::Int(43));
        assert!(!conj(g1, g2)(&mut store));
    }

    #[test]
    fn test_dfs_search() {
        let engine = SearchEngine::new(10);

        let g = |store: &mut ConstraintStore| {
            let x = store.fresh_var();
            disj(
                eq(x.clone(), LogicValue::Int(1)),
                eq(x.clone(), LogicValue::Int(2)),
            )(store)
        };

        let goal: Goal = Rc::new(g);
        let results = engine.find_all(&goal, &[]);
        assert!(!results.is_empty());
    }
}
