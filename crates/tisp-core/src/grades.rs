use crate::symbol::Symbol;
use crate::types::Grade;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResourceAlgebra {
    pub name: Symbol,
    pub semiring: Semiring,
    pub order: Order,
    pub asymptotic: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Semiring {
    pub add: Symbol,
    pub zero: Grade,
    pub mul: Symbol,
    pub one: Grade,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Order {
    Discrete,
    Total,
    Lattice(Vec<(Grade, Grade)>),
}

pub fn grade_add(a: &Grade, b: &Grade) -> Grade {
    match (a, b) {
        (Grade::Zero, r) | (r, Grade::Zero) => r.clone(),
        (Grade::Omega, _) | (_, Grade::Omega) => Grade::Omega,
        (Grade::Nat(m), Grade::Nat(n)) => Grade::Nat(m + n),
        (Grade::Nat(m), Grade::One) => Grade::Nat(m + 1),
        (Grade::One, Grade::Nat(n)) => Grade::Nat(n + 1),
        (Grade::One, Grade::One) => Grade::Nat(2),
        _ => Grade::Add(Box::new(a.clone()), Box::new(b.clone())),
    }
}

pub fn grade_mul(a: &Grade, b: &Grade) -> Grade {
    match (a, b) {
        (Grade::Zero, _) | (_, Grade::Zero) => Grade::Zero,
        (Grade::One, r) | (r, Grade::One) => r.clone(),
        (Grade::Omega, _) | (_, Grade::Omega) => Grade::Omega,
        (Grade::Nat(m), Grade::Nat(n)) => Grade::Nat(m * n),
        _ => Grade::Mul(Box::new(a.clone()), Box::new(b.clone())),
    }
}

pub fn grade_le(a: &Grade, b: &Grade) -> Option<bool> {
    match (a, b) {
        (Grade::Zero, _) => Some(true),
        (Grade::Omega, Grade::Omega) => Some(true),
        (_, Grade::Omega) => Some(true),
        (Grade::One, Grade::One) => Some(true),
        (Grade::Nat(m), Grade::Nat(n)) => Some(m <= n),
        _ => None,
    }
}

pub fn builtin_nat_algebra() -> ResourceAlgebra {
    ResourceAlgebra {
        name: Symbol::new("Nat"),
        semiring: Semiring {
            add: Symbol::new("+"),
            zero: Grade::Zero,
            mul: Symbol::new("*"),
            one: Grade::One,
        },
        order: Order::Total,
        asymptotic: false,
    }
}

pub fn builtin_sec_algebra() -> ResourceAlgebra {
    ResourceAlgebra {
        name: Symbol::new("Sec"),
        semiring: Semiring {
            add: Symbol::new("join"),
            zero: Grade::Custom(Symbol::new("Public"), Box::new(Grade::Zero)),
            mul: Symbol::new("meet"),
            one: Grade::Custom(Symbol::new("Private"), Box::new(Grade::One)),
        },
        order: Order::Lattice(vec![
            (Grade::Custom(Symbol::new("Public"), Box::new(Grade::Zero)),
             Grade::Custom(Symbol::new("Private"), Box::new(Grade::One))),
        ]),
        asymptotic: false,
    }
}
