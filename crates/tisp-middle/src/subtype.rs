//! §9 五维子类型格:effect/region/grade/mode/determinism 的子类型关系。
//! 除效果行子类型(§12.5,在 effect_infer 中)外,本模块提供 region/grade/mode/determinism
//! 的子类型判定,供类型检查按协变/逆变位置调用。

use tisp_core::determinism::{DetCategory, MaxSolutions};
use tisp_core::types::{Determinism, Grade, Mode};

/// 确定性子类型:det ≤ semidet ≤ multi ≤ nondet(Mercury 序)。
/// `a <: b` 表示 a 可安全用于期望 b 的位置(a 至少与 b 一样确定)。
pub fn det_subtype(a: Determinism, b: Determinism) -> bool {
    let da = DetCategory::from_det(&a);
    let db = DetCategory::from_det(&b);
    // a 可失败则 b 也须允许失败;a 的解数 ≤ b 的解数(Zero < One < Many)
    (!da.can_fail || db.can_fail) && max_le(da.max_solutions, db.max_solutions)
}

fn max_le(a: MaxSolutions, b: MaxSolutions) -> bool {
    use MaxSolutions::*;
    matches!(
        (a, b),
        (Zero, _) | (One, One) | (One, Many) | (Many, Many)
    )
}

/// 等级子类型:更宽松等级可作更严等级(ω <: r;有限等级间按半环上界)。
/// 即「用法更少」的值可安全用于「用法更多」的位置。
pub fn grade_subtype(a: Grade, b: Grade) -> bool {
    use Grade::*;
    match (a, b) {
        (Zero, _) => true,               // 0 级(擦除)可用于任何位置
        (Omega, _) => true,              // ω 不限,可用于任何位置
        (_, Omega) => true,              // 任何等级可用于 ω 期望位置
        (One, One) | (One, Nat(_)) => true,
        (Nat(m), Nat(n)) => m <= n,
        (Nat(_), One) => false,
        (Add(..), _) | (Mul(..), _) => false, // 复合等级不可静态判定,保守拒绝
        (Var(_), _) => true,             // 符号等级不可判定:放行(与等级检查策略一致)
        (Custom(..), _) => true,
        (One, Add(..)) | (One, Mul(..)) | (Nat(_), Add(..)) | (Nat(_), Mul(..)) => true,
        _ => false,
    }
}

/// 模式子类型:in <: out(输入模式可作输出模式的位置?)。保守:同模式或 in → 任意。
/// 当前 Mercury 多模式由 mode_analysis 按调用方向选择,此处提供基础子类型。
pub fn mode_subtype(a: Mode, b: Mode) -> bool {
    use Mode::*;
    match (a, b) {
        (In, _) => true, // in(ground)可作任何期望模式
        (x, y) => x == y,
    }
}

/// 区域子类型:子区域可作父区域。当前区域为名字型,同名字类型,不同名保守拒绝。
pub fn region_subtype(a: &tisp_core::types::RegionVar, b: &tisp_core::types::RegionVar) -> bool {
    a == b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_det_subtype() {
        assert!(det_subtype(Determinism::Det, Determinism::SemiDet));
        assert!(det_subtype(Determinism::Det, Determinism::NonDet));
        assert!(det_subtype(Determinism::SemiDet, Determinism::NonDet));
        assert!(det_subtype(Determinism::Multi, Determinism::NonDet));
        assert!(!det_subtype(Determinism::NonDet, Determinism::Det), "nondet 不可作 det");
        assert!(!det_subtype(Determinism::NonDet, Determinism::SemiDet), "nondet 不可作 semidet");
        assert!(!det_subtype(Determinism::Multi, Determinism::SemiDet), "multi 不可作 semidet");
        assert!(det_subtype(Determinism::Det, Determinism::Det));
    }

    #[test]
    fn test_grade_subtype() {
        assert!(grade_subtype(Grade::Zero, Grade::One));
        assert!(grade_subtype(Grade::Omega, Grade::One), "ω 可作更严等级");
        assert!(grade_subtype(Grade::Nat(1), Grade::Nat(3)));
        assert!(!grade_subtype(Grade::Nat(5), Grade::Nat(3)));
    }

    #[test]
    fn test_mode_subtype() {
        assert!(mode_subtype(Mode::In, Mode::Out));
        assert!(mode_subtype(Mode::In, Mode::In));
    }
}
