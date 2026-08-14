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

/// 从解析后的声明代数(types::ResourceAlgebra)构建语义模型(§11.1 接线)
pub fn from_declared(decl: &crate::types::ResourceAlgebra) -> ResourceAlgebra {
    let zero = match decl.unit.as_str() {
        "0" => Grade::Zero,
        "1" => Grade::One,
        s => Grade::Custom(Symbol::new(s), Box::new(Grade::Zero)),
    };
    let order = if decl.order.is_some() { Order::Total } else { Order::Discrete };
    ResourceAlgebra {
        name: decl.name.clone(),
        semiring: Semiring {
            add: decl.op.clone(),
            zero,
            mul: Symbol::new("*"),
            one: Grade::One,
        },
        order,
        asymptotic: decl.asymptotic,
    }
}

/// §11.1/§11.4 Cost 检查:在声明代数下比较代价上界 actual ≤ bound。
/// 可判定返回 Some(bool),不可判定(符号等级)返回 None(调用方据此警告放行)。
/// asymptotic 代数走渐近比较(忽略常数因子,O(n)+O(1)=O(n))。
pub fn check_cost_bound(alg: &ResourceAlgebra, actual: &Grade, bound: &Grade) -> Option<bool> {
    if alg.asymptotic {
        grade_le_asymptotic(actual, bound)
    } else {
        grade_le(actual, bound)
    }
}

/// §11.4 渐近(Big-O)代价比较:忽略常数因子,比较主导项。
fn grade_le_asymptotic(a: &Grade, b: &Grade) -> Option<bool> {
    use Grade::*;
    match (a, b) {
        // 常数项均为 O(1),等价
        (Nat(_), Nat(_)) => Some(true),
        // 同变量 O(n) ≤ O(n);不同变量不可判定
        (Var(x), Var(y)) => if x == y { Some(true) } else { None },
        // O(n) + O(1) = O(n):常数项不影响渐近
        (Add(l, _), Var(v)) => grade_le_asymptotic(l, &Var(v.clone())),
        (Var(v), Add(r, _)) => grade_le_asymptotic(&Var(v.clone()), r),
        (Zero, _) => Some(true),
        (_, Omega) => Some(true),
        // 乘/复合/不可判定:返回 None(警告放行)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_declared_wiring() {
        // §11.1 接线:声明代数 → 语义模型(Semiring/Order)
        let decl = crate::types::ResourceAlgebra {
            name: Symbol::new("Cost"),
            unit: "0".to_string(),
            op: Symbol::new("+"),
            order: Some(Symbol::new("<=")),
            asymptotic: true,
        };
        let alg = from_declared(&decl);
        assert_eq!(alg.name.as_str(), "Cost");
        assert_eq!(alg.semiring.add.as_str(), "+");
        assert_eq!(alg.semiring.zero, Grade::Zero);
        assert_eq!(alg.order, Order::Total);
        assert!(alg.asymptotic);
    }

    #[test]
    fn test_check_cost_bound() {
        // §11.4 Cost 检查(离散序):可判定比较返回 Some,符号等级返回 None(警告放行)
        let decl = crate::types::ResourceAlgebra {
            name: Symbol::new("Cost"),
            unit: "0".to_string(),
            op: Symbol::new("+"),
            order: Some(Symbol::new("<=")),
            asymptotic: false,
        };
        let alg = from_declared(&decl);
        assert_eq!(check_cost_bound(&alg, &Grade::Nat(3), &Grade::Nat(5)), Some(true));
        assert_eq!(check_cost_bound(&alg, &Grade::Nat(7), &Grade::Nat(5)), Some(false));
        // 符号等级不可判定
        assert_eq!(check_cost_bound(&alg, &Grade::Var(Symbol::new("n")), &Grade::Nat(5)), None);
    }

    #[test]
    fn test_check_cost_bound_asymptotic() {
        // §11.4 渐近代价:忽略常数因子,O(n)+O(1)=O(n);常数项均等价
        let decl = crate::types::ResourceAlgebra {
            name: Symbol::new("Cost"),
            unit: "0".to_string(),
            op: Symbol::new("+"),
            order: Some(Symbol::new("<=")),
            asymptotic: true,
        };
        let alg = from_declared(&decl);
        // 常数项渐近等价(5 ≤ 3 在 Big-O 下为 true)
        assert_eq!(check_cost_bound(&alg, &Grade::Nat(5), &Grade::Nat(3)), Some(true));
        // O(n) + O(1) ≤ O(n)
        assert_eq!(check_cost_bound(&alg, &Grade::Add(Box::new(Grade::Var(Symbol::new("n"))), Box::new(Grade::Nat(1))), &Grade::Var(Symbol::new("n"))), Some(true));
        // O(n) ≤ O(n)
        assert_eq!(check_cost_bound(&alg, &Grade::Var(Symbol::new("n")), &Grade::Var(Symbol::new("n"))), Some(true));
        // O(n) ≤ O(m) 不可判定(不同变量)
        assert_eq!(check_cost_bound(&alg, &Grade::Var(Symbol::new("n")), &Grade::Var(Symbol::new("m"))), None);
    }
}
