/// Abductive Logic Programming (ALP) runtime
/// Generates hypotheses and checks consistency with known constraints
use std::collections::HashMap;

/// An abduction hypothesis — a binding for an abducible variable
#[derive(Debug, Clone, PartialEq)]
pub struct Hypothesis {
    pub var: String,
    pub value: i64,
}

/// An abduction explanation — a set of consistent hypotheses
#[derive(Debug, Clone)]
pub struct Explanation {
    pub hypotheses: Vec<Hypothesis>,
}

/// Abduction engine: generate explanations for a goal given abducible variables
pub struct AbductionEngine {
    max_hypotheses: usize,
}

impl AbductionEngine {
    pub fn new() -> Self {
        Self { max_hypotheses: 10 }
    }

    /// Generate candidate hypotheses for abducible variables
    pub fn generate_hypotheses(&mut self, abducibles: &[String], domains: &HashMap<String, (i64, i64)>) -> Vec<Explanation> {
        let mut explanations = Vec::new();
        let mut current = Vec::new();
        self.generate_recursive(abducibles, domains, 0, &mut current, &mut explanations);
        explanations
    }

    fn generate_recursive(
        &self,
        abducibles: &[String],
        domains: &HashMap<String, (i64, i64)>,
        idx: usize,
        current: &mut Vec<Hypothesis>,
        results: &mut Vec<Explanation>,
    ) {
        if idx >= abducibles.len() || current.len() >= self.max_hypotheses {
            if !current.is_empty() {
                results.push(Explanation { hypotheses: current.clone() });
            }
            return;
        }
        let var = &abducibles[idx];
        if let Some((lo, hi)) = domains.get(var) {
            for v in *lo..=*hi {
                current.push(Hypothesis { var: var.clone(), value: v });
                self.generate_recursive(abducibles, domains, idx + 1, current, results);
                current.pop();
            }
        } else {
            // No domain info — generate heuristic candidates
            for v in 0..=5 {
                current.push(Hypothesis { var: var.clone(), value: v });
                self.generate_recursive(abducibles, domains, idx + 1, current, results);
                current.pop();
            }
        }
    }

    /// Check if an explanation is consistent with a constraint function
    pub fn check_consistent(&self, exp: &Explanation, constraint: &dyn Fn(&Hypothesis) -> bool) -> bool {
        exp.hypotheses.iter().all(|h| constraint(h))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abduce_simple() {
        let mut engine = AbductionEngine::new();
        let abducibles = vec!["x".to_string(), "y".to_string()];
        let mut domains = HashMap::new();
        domains.insert("x".to_string(), (1, 3));
        domains.insert("y".to_string(), (0, 1));
        let exps = engine.generate_hypotheses(&abducibles, &domains);
        assert!(exps.len() > 0);
        for exp in &exps {
            assert_eq!(exp.hypotheses.len(), 2);
        }
    }

    #[test]
    fn test_abduce_empty() {
        let mut engine = AbductionEngine::new();
        let exps = engine.generate_hypotheses(&[], &HashMap::new());
        assert_eq!(exps.len(), 0);
    }
}
