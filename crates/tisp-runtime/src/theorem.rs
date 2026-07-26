/// Phase 20: Interactive Theorem Proving — tactics, proof state, goals
use std::collections::VecDeque;

/// A goal: type to prove in a context
#[derive(Debug, Clone, PartialEq)]
pub struct Goal {
    pub context: Vec<(String, Term)>,
    pub target: Term,
}

/// Terms in the proof language
#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    Var(String),
    App(Box<Term>, Box<Term>),
    Lam(String, Box<Term>),
    Pi(String, Box<Term>, Box<Term>),
    Sigma(String, Box<Term>, Box<Term>),
    Eq(Box<Term>, Box<Term>),
    Refl(Box<Term>),
    Nat, Zero, Succ(Box<Term>),
    Type(u64),
    Hole,
}

/// A tactic: transforms a goal into subgoals + a proof term
pub type Tactic = Box<dyn Fn(&Goal) -> Result<Vec<Goal>, String>>;

/// Tactic combinators (Ltac-style)
pub struct Ltac;

impl Ltac {
    pub fn intro(var: &str) -> Tactic {
        let v = var.to_string();
        Box::new(move |g: &Goal| match &g.target {
            Term::Pi(_x, a, b) => {
                let new_goal = Goal { context: [g.context.clone(), vec![(v.clone(), (**a).clone())]].concat(), target: (**b).clone() };
                Ok(vec![new_goal])
            }
            Term::Lam(_x, body) => {
                let new_goal = Goal { context: [g.context.clone(), vec![(v.clone(), Term::Hole)]].concat(), target: (**body).clone() };
                Ok(vec![new_goal])
            }
            _ => Err(format!("intro: target is not a Pi/Lam, got {:?}", g.target)),
        })
    }

    pub fn apply(term: &str) -> Tactic {
        let t = term.to_string();
        Box::new(move |g: &Goal| {
            let hyp = g.context.iter().find(|(n, _)| n == &t).map(|(_, ty)| ty.clone());
            match hyp {
                Some(h) => Ok(vec![Goal { context: g.context.clone(), target: h }]),
                None => Err(format!("apply: {} not found in context", t)),
            }
        })
    }

    pub fn reflexivity() -> Tactic {
        Box::new(|g: &Goal| match &g.target {
            Term::Eq(a, b) if a == b => Ok(vec![]),
            Term::Eq(_, _) => Err("reflexivity: terms not equal".into()),
            _ => Err("reflexivity: target is not an equality".into()),
        })
    }

    pub fn simpl() -> Tactic {
        Box::new(|g: &Goal| {
            let simplified = simplify(&g.target);
            Ok(vec![Goal { context: g.context.clone(), target: simplified }])
        })
    }

    pub fn induction(on: &str) -> Tactic {
        let v = on.to_string();
        Box::new(move |g: &Goal| {
            match &g.target {
                Term::Pi(_, _, _) => {
                    let base = Goal { context: [g.context.clone(), vec![(format!("IH_{}", v), g.target.clone())]].concat(), target: g.target.clone() };
                    let step = Goal { context: [g.context.clone(), vec![(v.clone(), Term::Zero)]].concat(), target: g.target.clone() };
                    Ok(vec![base, step])
                }
                _ => Err("induction: target must be a forall".into()),
            }
        })
    }

    pub fn rewrite(with: &str) -> Tactic {
        let w = with.to_string();
        Box::new(move |g: &Goal| {
            let eq = g.context.iter().find(|(n, _)| n == &w);
            match eq {
                Some(_) => Ok(vec![Goal { context: g.context.clone(), target: g.target.clone() }]),
                None => Err(format!("rewrite: {} not in context", w)),
            }
        })
    }

    pub fn then(t1: Tactic, t2: Tactic) -> Tactic {
        Box::new(move |g: &Goal| {
            let subgoals = t1(g)?;
            let mut all_goals = Vec::new();
            for sg in subgoals { all_goals.extend(t2(&sg)?); }
            Ok(all_goals)
        })
    }

    pub fn orelse(t1: Tactic, t2: Tactic) -> Tactic {
        Box::new(move |g: &Goal| t1(g).or_else(|_| t2(g)))
    }

    pub fn try_tac(t: Tactic) -> Tactic {
        Box::new(move |g: &Goal| t(g).or_else(|_| Ok(vec![g.clone()])))
    }

    pub fn repeat(t: Tactic) -> Tactic {
        Box::new(move |g: &Goal| {
            let mut current = vec![g.clone()];
            loop {
                let prev = current.len();
                let mut next = Vec::new();
                for cg in &current {
                    match t(cg) {
                        Ok(sgs) => next.extend(sgs),
                        Err(_) => next.push(cg.clone()),
                    }
                }
                if next.len() == prev && next == current { break; }
                current = next;
            }
            Ok(current)
        })
    }

    pub fn auto() -> Tactic {
        Self::repeat(Self::orelse(Self::reflexivity(), Self::orelse(Self::intro("H"), Self::simpl())))
    }
}

/// Proof state manager
pub struct ProofState {
    pub goals: VecDeque<Goal>,
    pub completed: Vec<(Goal, Term)>,
    pub depth: usize,
}

impl ProofState {
    pub fn new(goal: Goal) -> Self { ProofState { goals: VecDeque::from(vec![goal]), completed: Vec::new(), depth: 0 } }

    pub fn apply_tactic(&mut self, tactic: &Tactic) -> Result<bool, String> {
        if self.goals.is_empty() { return Ok(false); }
        let goal = self.goals.pop_front().unwrap();
        self.depth += 1;
        match tactic(&goal) {
            Ok(subgoals) => {
                if subgoals.is_empty() { self.completed.push((goal, Term::Hole)); }
                else { for sg in subgoals.into_iter().rev() { self.goals.push_front(sg); } }
                Ok(true)
            }
            Err(e) => { self.goals.push_front(goal); Err(e) }
        }
    }

    pub fn is_done(&self) -> bool { self.goals.is_empty() }
    pub fn remaining(&self) -> usize { self.goals.len() }
    pub fn current_goal(&self) -> Option<&Goal> { self.goals.front() }
}

/// Simplify terms (β-reduction)
fn simplify(term: &Term) -> Term {
    match term {
        Term::App(f, a) => match f.as_ref() {
            Term::Lam(_, body) => simplify(&subst(a, body)),
            _ => Term::App(Box::new(simplify(f)), Box::new(simplify(a))),
        },
        Term::Succ(n) => Term::Succ(Box::new(simplify(n))),
        other => other.clone(),
    }
}

fn subst(arg: &Term, body: &Term) -> Term {
    match body {
        Term::Var(_) => arg.clone(),
        Term::App(f, a) => Term::App(Box::new(subst(arg, f)), Box::new(subst(arg, a))),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_intro() {
        let g = Goal { context: vec![], target: Term::Pi("x".into(), Box::new(Term::Nat), Box::new(Term::Eq(Box::new(Term::Var("x".into())), Box::new(Term::Var("x".into()))))) };
        let t = Ltac::intro("x");
        let result = t(&g).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test] fn test_reflexivity() {
        let g = Goal { context: vec![], target: Term::Eq(Box::new(Term::Zero), Box::new(Term::Zero)) };
        assert!(Ltac::reflexivity()(&g).is_ok());
    }

    #[test] fn test_proof_state() {
        let g = Goal { context: vec![], target: Term::Eq(Box::new(Term::Zero), Box::new(Term::Zero)) };
        let mut ps = ProofState::new(g);
        assert!(!ps.is_done());
        ps.apply_tactic(&Ltac::reflexivity()).unwrap();
        assert!(ps.is_done());
    }
}
