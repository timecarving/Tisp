use crate::types::Determinism;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct DetCategory {
    pub can_fail: bool,
    pub max_solutions: MaxSolutions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MaxSolutions {
    Zero,
    One,
    Many,
}

impl DetCategory {
    pub fn from_det(d: &Determinism) -> Self {
        match d {
            Determinism::Det => Self { can_fail: false, max_solutions: MaxSolutions::One },
            Determinism::SemiDet => Self { can_fail: true, max_solutions: MaxSolutions::One },
            Determinism::Multi => Self { can_fail: false, max_solutions: MaxSolutions::Many },
            Determinism::NonDet => Self { can_fail: true, max_solutions: MaxSolutions::Many },
            Determinism::CcMulti => Self { can_fail: false, max_solutions: MaxSolutions::One },
            Determinism::CcNonDet => Self { can_fail: true, max_solutions: MaxSolutions::One },
            Determinism::Failure => Self { can_fail: true, max_solutions: MaxSolutions::Zero },
            Determinism::Erroneous => Self { can_fail: true, max_solutions: MaxSolutions::Zero },
        }
    }

    pub fn to_det(&self) -> Determinism {
        match (self.can_fail, self.max_solutions) {
            (false, MaxSolutions::One) => Determinism::Det,
            (true, MaxSolutions::One) => Determinism::SemiDet,
            (false, MaxSolutions::Many) => Determinism::Multi,
            (true, MaxSolutions::Many) => Determinism::NonDet,
            (true, MaxSolutions::Zero) => Determinism::Failure,
            (false, MaxSolutions::Zero) => Determinism::Det,
        }
    }
}

pub fn det_conjunction(a: &DetCategory, b: &DetCategory) -> DetCategory {
    let can_fail = a.can_fail || (!a.can_fail && b.can_fail);
    let max_solutions = match (a.max_solutions, b.max_solutions) {
        (MaxSolutions::Zero, _) | (_, MaxSolutions::Zero) => MaxSolutions::Zero,
        (MaxSolutions::One, MaxSolutions::One) => MaxSolutions::One,
        _ => MaxSolutions::Many,
    };
    DetCategory { can_fail, max_solutions }
}

pub fn det_disjunction(a: &DetCategory, b: &DetCategory) -> DetCategory {
    let can_fail = a.can_fail && b.can_fail;
    let max_solutions = match (a.max_solutions, b.max_solutions) {
        (MaxSolutions::Zero, MaxSolutions::Zero) => MaxSolutions::Zero,
        (MaxSolutions::Zero, other) | (other, MaxSolutions::Zero) => other,
        _ => MaxSolutions::Many,
    };
    DetCategory { can_fail, max_solutions }
}

pub fn det_negation(a: &DetCategory) -> DetCategory {
    match (a.can_fail, a.max_solutions) {
        (true, MaxSolutions::Zero) => DetCategory { can_fail: false, max_solutions: MaxSolutions::One },
        (false, _) | (_, MaxSolutions::Many) => DetCategory { can_fail: true, max_solutions: MaxSolutions::Zero },
        _ => DetCategory { can_fail: true, max_solutions: MaxSolutions::One },
    }
}
