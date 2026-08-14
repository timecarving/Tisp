/// Constraint Logic Programming (CLP) runtime
/// Supports CLP(FD) — Constraint Logic Programming over Finite Domains

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::rc::Rc;

/// A finite domain for constraint variables(有序集合,label 按升序枚举解)
#[derive(Debug, Clone, PartialEq)]
pub struct Domain {
    values: BTreeSet<i64>,
}

impl Domain {
    pub fn range(lo: i64, hi: i64) -> Self {
        Self { values: (lo..=hi).collect() }
    }

    pub fn from_slice(values: &[i64]) -> Self {
        Self { values: values.iter().copied().collect() }
    }

    pub fn singleton(v: i64) -> Self {
        let mut s = BTreeSet::new(); s.insert(v); Self { values: s }
    }

    pub fn size(&self) -> usize { self.values.len() }

    pub fn is_empty(&self) -> bool { self.values.is_empty() }

    pub fn min(&self) -> Option<i64> { self.values.iter().next().copied() }
    pub fn max(&self) -> Option<i64> { self.values.iter().next_back().copied() }

    pub fn remove(&mut self, v: i64) -> bool { self.values.remove(&v) }
    pub fn retain(&mut self, pred: impl Fn(&i64) -> bool) { self.values.retain(pred); }
    pub fn contains(&self, v: i64) -> bool { self.values.contains(&v) }
    pub fn iter(&self) -> impl Iterator<Item = &i64> { self.values.iter() }
    pub fn single_value(&self) -> Option<i64> {
        if self.values.len() == 1 { self.values.iter().next().copied() } else { None }
    }
}

/// A constraint propagator
type Propagator = Rc<dyn Fn(&mut ConstraintStore) -> bool>;

/// Constraint store with domains and propagators
#[derive(Clone)]
pub struct ConstraintStore {
    domains: HashMap<u64, Domain>,
    propagators: Vec<Propagator>,
    next_var_id: u64,
    /// Trail for backtracking: (var_id, old_domain)
    trail: Vec<(u64, Domain)>,
}

impl ConstraintStore {
    pub fn new() -> Self {
        Self { domains: HashMap::new(), propagators: Vec::new(), next_var_id: 0, trail: Vec::new() }
    }

    /// Create a new finite domain variable
    pub fn new_var(&mut self, domain: Domain) -> u64 {
        let id = self.next_var_id; self.next_var_id += 1;
        self.domains.insert(id, domain);
        id
    }

    /// Create a new variable with range domain
    pub fn new_int_var(&mut self, lo: i64, hi: i64) -> u64 {
        self.new_var(Domain::range(lo, hi))
    }

    /// Get domain of a variable
    pub fn domain_of(&self, var: u64) -> Option<&Domain> {
        self.domains.get(&var)
    }

    /// Update domain of a variable (returns true if changed)
    fn update_domain(&mut self, var: u64, new_domain: Domain) -> bool {
        if let Some(old) = self.domains.get(&var) {
            if *old != new_domain {
                self.trail.push((var, old.clone()));
                self.domains.insert(var, new_domain);
                return true;
            }
        }
        false
    }

    /// Add a propagator
    pub fn add_propagator(&mut self, prop: Propagator) {
        self.propagators.push(prop);
    }

    /// Add inequality constraint: x < y
    pub fn add_lt(&mut self, x: u64, y: u64) {
        let prop: Propagator = Rc::new(move |store: &mut ConstraintStore| {
            let _x_max = store.domain_of(x).and_then(|d| d.max()).unwrap_or(i64::MAX);
            let _y_min = store.domain_of(y).and_then(|d| d.min()).unwrap_or(i64::MIN);
            let mut changed = false;
            // x < y: all values of x must be < y_max (there exists some y > x)
            //        all values of y must be > x_min (there exists some x < y)
            if let Some(y_dom) = store.domain_of(y) {
                let y_max_val = y_dom.max().unwrap_or(i64::MAX);
                if let Some(x_dom) = store.domain_of(x).cloned() {
                    let mut new_x = x_dom.clone();
                    new_x.retain(|v| *v < y_max_val);
                    if new_x != x_dom { changed |= store.update_domain(x, new_x); }
                }
            }
            if let Some(x_dom) = store.domain_of(x) {
                let x_min_val = x_dom.min().unwrap_or(i64::MIN);
                if let Some(y_dom) = store.domain_of(y).cloned() {
                    let mut new_y = y_dom.clone();
                    new_y.retain(|v| *v > x_min_val);
                    if new_y != y_dom { changed |= store.update_domain(y, new_y); }
                }
            }
            changed
        });
        self.propagators.push(prop);
    }

    /// Add equality constraint: x = y
    pub fn add_eq(&mut self, x: u64, y: u64) {
        let prop: Propagator = Rc::new(move |store: &mut ConstraintStore| {
            let x_dom = store.domain_of(x).cloned().unwrap_or(Domain::range(0, 0));
            let y_dom = store.domain_of(y).cloned().unwrap_or(Domain::range(0, 0));
            let intersection = {
                let mut set = BTreeSet::new();
                for v in x_dom.iter() {
                    if y_dom.contains(*v) { set.insert(*v); }
                }
                set
            };
            let new_dom = Domain { values: intersection };
            let mut changed = false;
            // 空域也更新(空域 = 冲突信号,由 has_empty_domain 检测)
            if let Some(_) = store.domain_of(x) { changed |= store.update_domain(x, new_dom.clone()); }
            if let Some(_) = store.domain_of(y) { changed |= store.update_domain(y, new_dom); }
            changed
        });
        self.propagators.push(prop);
    }

    /// Add all_different constraint
    pub fn add_all_different(&mut self, vars: &[u64]) {
        let vars = vars.to_vec();
        let prop: Propagator = Rc::new(move |store: &mut ConstraintStore| {
            let mut changed = false;
            // Find assigned (singleton) variables
            let assigned: Vec<(usize, i64)> = vars.iter().enumerate()
                .filter_map(|(i, v)| store.domain_of(*v).and_then(|d| d.single_value()).map(|val| (i, val)))
                .collect();
            // Remove assigned values from other domains
            for (ai, val) in &assigned {
                for (j, vj) in vars.iter().enumerate() {
                    if j == *ai { continue; }
                    if let Some(dom) = store.domain_of(*vj).cloned() {
                        if dom.contains(*val) && dom.size() > 1 {
                            let mut new_dom = dom.clone();
                            new_dom.remove(*val);
                            changed |= store.update_domain(*vj, new_dom);
                        }
                    }
                }
            }
            changed
        });
        self.propagators.push(prop);
    }

    /// Add multiplication constraint: x * y = z(域枚举收缩,教学级)
    pub fn add_mul(&mut self, x: u64, y: u64, z: u64) {
        let prop: Propagator = Rc::new(move |store: &mut ConstraintStore| {
            let mut changed = false;
            let x_dom = store.domain_of(x).cloned().unwrap_or(Domain::range(0, 0));
            let y_dom = store.domain_of(y).cloned().unwrap_or(Domain::range(0, 0));
            let z_dom = store.domain_of(z).cloned().unwrap_or(Domain::range(0, 0));
            // x 域收缩:存在 y、z 使 x*y = z
            let new_x = {
                let mut set = BTreeSet::new();
                for v in x_dom.iter() {
                    if y_dom.iter().any(|w| z_dom.contains(v * w)) { set.insert(*v); }
                }
                Domain { values: set }
            };
            if new_x != x_dom { changed |= store.update_domain(x, new_x); }
            let x_dom2 = store.domain_of(x).cloned().unwrap_or(Domain::range(0, 0));
            let new_y = {
                let mut set = BTreeSet::new();
                for w in y_dom.iter() {
                    if x_dom2.iter().any(|v| z_dom.contains(v * w)) { set.insert(*w); }
                }
                Domain { values: set }
            };
            if new_y != y_dom { changed |= store.update_domain(y, new_y); }
            // z 域也收缩:z 必须是 x*y 的可能值(结果变量收窄,§21.5)
            let x_dom3 = store.domain_of(x).cloned().unwrap_or(Domain::range(0, 0));
            let y_dom3 = store.domain_of(y).cloned().unwrap_or(Domain::range(0, 0));
            let new_z = {
                let mut set = BTreeSet::new();
                for v in x_dom3.iter() {
                    for w in y_dom3.iter() {
                        set.insert(v * w);
                    }
                }
                Domain { values: set }
            };
            if new_z != z_dom { changed |= store.update_domain(z, new_z); }
            changed
        });
        self.propagators.push(prop);
    }

    /// Add addition constraint: x + y = z(域枚举收缩)
    pub fn add_plus(&mut self, x: u64, y: u64, z: u64) {
        let prop: Propagator = Rc::new(move |store: &mut ConstraintStore| {
            let mut changed = false;
            let x_dom = store.domain_of(x).cloned().unwrap_or(Domain::range(0, 0));
            let y_dom = store.domain_of(y).cloned().unwrap_or(Domain::range(0, 0));
            let z_dom = store.domain_of(z).cloned().unwrap_or(Domain::range(0, 0));
            let new_x = {
                let mut set = BTreeSet::new();
                for v in x_dom.iter() {
                    if y_dom.iter().any(|w| z_dom.contains(v + w)) { set.insert(*v); }
                }
                Domain { values: set }
            };
            if new_x != x_dom { changed |= store.update_domain(x, new_x); }
            let x2 = store.domain_of(x).cloned().unwrap_or(Domain::range(0, 0));
            let new_y = {
                let mut set = BTreeSet::new();
                for w in y_dom.iter() {
                    if x2.iter().any(|v| z_dom.contains(v + w)) { set.insert(*w); }
                }
                Domain { values: set }
            };
            if new_y != y_dom { changed |= store.update_domain(y, new_y); }
            let y2 = store.domain_of(y).cloned().unwrap_or(Domain::range(0, 0));
            let new_z = {
                let mut set = BTreeSet::new();
                for v in x2.iter() {
                    for w in y2.iter() {
                        set.insert(v + w);
                    }
                }
                Domain { values: set }
            };
            if new_z != z_dom { changed |= store.update_domain(z, new_z); }
            changed
        });
        self.propagators.push(prop);
    }

    /// Add subtraction constraint: x - y = z(域枚举收缩)
    pub fn add_minus(&mut self, x: u64, y: u64, z: u64) {
        let prop: Propagator = Rc::new(move |store: &mut ConstraintStore| {
            let mut changed = false;
            let x_dom = store.domain_of(x).cloned().unwrap_or(Domain::range(0, 0));
            let y_dom = store.domain_of(y).cloned().unwrap_or(Domain::range(0, 0));
            let z_dom = store.domain_of(z).cloned().unwrap_or(Domain::range(0, 0));
            let new_x = {
                let mut set = BTreeSet::new();
                for v in x_dom.iter() {
                    if y_dom.iter().any(|w| z_dom.contains(v - w)) { set.insert(*v); }
                }
                Domain { values: set }
            };
            if new_x != x_dom { changed |= store.update_domain(x, new_x); }
            let x2 = store.domain_of(x).cloned().unwrap_or(Domain::range(0, 0));
            let new_y = {
                let mut set = BTreeSet::new();
                for w in y_dom.iter() {
                    if x2.iter().any(|v| z_dom.contains(v - w)) { set.insert(*w); }
                }
                Domain { values: set }
            };
            if new_y != y_dom { changed |= store.update_domain(y, new_y); }
            let y2 = store.domain_of(y).cloned().unwrap_or(Domain::range(0, 0));
            let new_z = {
                let mut set = BTreeSet::new();
                for v in x2.iter() {
                    for w in y2.iter() {
                        set.insert(v - w);
                    }
                }
                Domain { values: set }
            };
            if new_z != z_dom { changed |= store.update_domain(z, new_z); }
            changed
        });
        self.propagators.push(prop);
    }

    /// Add division constraint: x / y = z,即 x = y * z(域枚举收缩)
    pub fn add_div(&mut self, x: u64, y: u64, z: u64) {
        let prop: Propagator = Rc::new(move |store: &mut ConstraintStore| {
            let mut changed = false;
            let x_dom = store.domain_of(x).cloned().unwrap_or(Domain::range(0, 0));
            let y_dom = store.domain_of(y).cloned().unwrap_or(Domain::range(0, 0));
            let z_dom = store.domain_of(z).cloned().unwrap_or(Domain::range(0, 0));
            let new_x = {
                let mut set = BTreeSet::new();
                for v in x_dom.iter() {
                    // 精确除法:须整除(v % w == 0),不把截断值判为满足(§21.5)
                    if y_dom.iter().any(|w| *w != 0 && v % w == 0 && z_dom.contains(v / w)) { set.insert(*v); }
                }
                Domain { values: set }
            };
            if new_x != x_dom { changed |= store.update_domain(x, new_x); }
            let x_dom2 = store.domain_of(x).cloned().unwrap_or(Domain::range(0, 0));
            let new_y = {
                let mut set = BTreeSet::new();
                for w in y_dom.iter() {
                    if *w != 0 && x_dom2.iter().any(|v| v % w == 0 && z_dom.contains(v / w)) { set.insert(*w); }
                }
                Domain { values: set }
            };
            if new_y != y_dom { changed |= store.update_domain(y, new_y); }
            let x_dom3 = store.domain_of(x).cloned().unwrap_or(Domain::range(0, 0));
            let y_dom3 = store.domain_of(y).cloned().unwrap_or(Domain::range(0, 0));
            let new_z = {
                let mut set = BTreeSet::new();
                for v in x_dom3.iter() {
                    for w in y_dom3.iter() {
                        if *w != 0 && v % w == 0 { set.insert(v / w); }
                    }
                }
                Domain { values: set }
            };
            if new_z != z_dom { changed |= store.update_domain(z, new_z); }
            changed
        });
        self.propagators.push(prop);
    }

    /// Add modulo constraint: x mod y = z(域枚举收缩)
    pub fn add_mod(&mut self, x: u64, y: u64, z: u64) {
        let prop: Propagator = Rc::new(move |store: &mut ConstraintStore| {
            let mut changed = false;
            let x_dom = store.domain_of(x).cloned().unwrap_or(Domain::range(0, 0));
            let y_dom = store.domain_of(y).cloned().unwrap_or(Domain::range(0, 0));
            let z_dom = store.domain_of(z).cloned().unwrap_or(Domain::range(0, 0));
            let new_x = {
                let mut set = BTreeSet::new();
                for v in x_dom.iter() {
                    if y_dom.iter().any(|w| *w != 0 && z_dom.contains(v % w)) { set.insert(*v); }
                }
                Domain { values: set }
            };
            if new_x != x_dom { changed |= store.update_domain(x, new_x); }
            changed
        });
        self.propagators.push(prop);
    }

    /// 是否存在空域(约束冲突)
    pub fn has_empty_domain(&self) -> bool {
        self.domains.values().any(|d| d.is_empty())
    }

    /// 单值赋值(提交解,供 label 提交模式)
    /// §21.6 域相交:值在当前域内则收窄为单值;越界则置空域(冲突信号,排除越界假设)
    pub fn assign(&mut self, id: u64, v: i64) {
        match self.domain_of(id).cloned() {
            Some(dom) => {
                if dom.contains(v) {
                    self.update_domain(id, Domain::singleton(v));
                } else {
                    self.update_domain(id, Domain::from_slice(&[]));
                }
            }
            None => {
                self.domains.insert(id, Domain::singleton(v));
            }
        }
    }

    /// 域快照(诊断)
    pub fn domains_snapshot(&self) -> Vec<(u64, Vec<i64>)> {
        self.domains.iter().map(|(id, d)| (*id, d.iter().copied().collect())).collect()
    }

    /// Run propagation loop (AC-3 style) until fixpoint
    pub fn propagate(&mut self) -> bool {
        let mut changed = true;
        let mut iterations = 0;
        let max_iterations = 1000;
        while changed && iterations < max_iterations {
            changed = false;
            iterations += 1;
            let props = self.propagators.clone();
            for prop in &props {
                if prop(self) { changed = true; }
            }
            // Check for empty domains
            if self.domains.values().any(|d| d.is_empty()) { return false; }
        }
        true
    }

    /// Labeling: assign values to variables by searching
    pub fn label(&mut self, vars: &[u64], results: &mut Vec<HashMap<u64, i64>>) -> bool {
        if vars.is_empty() {
            // All vars labeled — extract solution
            let solution: HashMap<u64, i64> = self.domains.iter()
                .filter_map(|(id, dom)| dom.single_value().map(|v| (*id, v)))
                .collect();
            results.push(solution);
            return true;
        }
        let var = vars[0];
        let domain = self.domain_of(var).cloned().unwrap_or(Domain::range(0, 0));
        let values: Vec<i64> = domain.iter().copied().collect();
        for value in values {
            let trail_depth = self.trail.len();
            self.update_domain(var, Domain::singleton(value));
            if self.propagate() {
                self.label(&vars[1..], results);
            }
            // Backtrack
            while self.trail.len() > trail_depth {
                let (vid, old_dom) = self.trail.pop().unwrap();
                self.domains.insert(vid, old_dom);
            }
        }
        !results.is_empty()
    }

    /// Get the trail depth
    pub fn trail_depth(&self) -> usize { self.trail.len() }
}

/// Abductive logic programming runtime
pub struct AbductiveEngine {
    abducibles: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct Explanation {
    pub assumptions: Vec<(String, Vec<i64>)>, // (predicate_name, args)
}

impl AbductiveEngine {
    pub fn new() -> Self { Self { abducibles: HashSet::new() } }

    /// Register an abducible predicate
    pub fn declare_abducible(&mut self, name: &str) {
        self.abducibles.insert(name.to_string());
    }

    /// Check if a predicate is abducible
    pub fn is_abducible(&self, name: &str) -> bool { self.abducibles.contains(name) }

    /// Find minimal explanations (by assumption count)
    pub fn find_explanations(
        &self,
        goal_check: impl Fn(&[Explanation]) -> bool,
        max_assumptions: usize,
    ) -> Vec<Explanation> {
        let mut results = Vec::new();
        // Simple BFS over assumption space
        let mut queue = VecDeque::new();
        queue.push_back(Explanation { assumptions: Vec::new() });
        while let Some(current) = queue.pop_front() {
            if current.assumptions.len() >= max_assumptions { continue; }
            if goal_check(&[current.clone()]) {
                results.push(current);
                continue; // Found explanation, don't expand further
            }
            // Try adding each abducible as a new assumption
            for abducible in &self.abducibles {
                let mut expanded = current.clone();
                expanded.assumptions.push((abducible.clone(), vec![]));
                queue.push_back(expanded);
            }
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fd_domain() {
        let dom = Domain::range(0, 10);
        assert_eq!(dom.size(), 11);
        assert!(dom.contains(5));
        assert!(!dom.contains(11));
    }

    #[test]
    fn test_propagation() {
        let mut store = ConstraintStore::new();
        let x = store.new_int_var(0, 10);
        let y = store.new_int_var(0, 10);
        store.add_lt(x, y);
        assert!(store.propagate());
        // After x < y propagation: x max < y min → x < 10, y > 0
        assert!(store.domain_of(x).unwrap().max().unwrap() < 10);
        assert!(store.domain_of(y).unwrap().min().unwrap() > 0);
    }

    #[test]
    fn test_labeling() {
        let mut store = ConstraintStore::new();
        let x = store.new_int_var(0, 3);
        let y = store.new_int_var(0, 3);
        store.add_lt(x, y);
        store.propagate();
        let mut results = Vec::new();
        store.label(&[x, y], &mut results);
        assert!(!results.is_empty());
        for sol in &results {
            assert!(sol.get(&x).unwrap() < sol.get(&y).unwrap());
        }
    }

    #[test]
    fn test_abductive() {
        let mut engine = AbductiveEngine::new();
        engine.declare_abducible("father");
        engine.declare_abducible("mother");
        let results = engine.find_explanations(|exps| exps.len() >= 1, 3);
        assert!(!results.is_empty());
    }
}
