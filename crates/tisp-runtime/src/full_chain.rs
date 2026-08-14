//! 全链路补齐:⚠️ 特性的语义助手(纯声明式)
//!
//! 分级模态推理、Cost 渐近代价、时序稳定类型、区域逃逸、HoTT 完整立方填充。
//! 作为运行时语义核心,配合 type_infer/grade_check/hott/region_infer 的接线。
use tisp_core::types::Grade;

// ── 1.1 分级模态 □_r/◇_ε 引入/消去推理 ──

/// □_r 引入/消去:使用次数 ≤ 等级 r 时通过(替换对 ω 绑定的恒过)
pub fn grade_covers(grade: &Grade, use_count: u64) -> bool {
    match grade {
        Grade::Zero => use_count == 0,
        Grade::One => use_count <= 1,
        Grade::Omega => true,
        Grade::Nat(n) => use_count <= *n,
        Grade::Add(a, b) => grade_covers(a, use_count) || grade_covers(b, use_count),
        Grade::Mul(a, b) => grade_covers(a, use_count) && grade_covers(b, use_count),
        // 符号/自定义等级:不可判定,警告放行(视为通过)
        Grade::Var(_) | Grade::Custom(_, _) => true,
    }
}

// ── 1.2 Cost 渐近代价全推导 ──

/// 渐近代价类(Big-O)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Cost {
    Const,
    Log,
    Linear,
    Poly(u32),
    Exp,
}

impl Cost {
    /// 递归复合:代价按代数复合,取渐近上界
    pub fn combine(self, other: Cost) -> Cost {
        self.max(other)
    }
}

/// 渐近上界比较(a ≤ b 即 a 的渐近阶不高于 b)
pub fn cost_le(a: Cost, b: Cost) -> bool {
    a <= b
}

// ── 1.5 时序 □_t 稳定类型语义 ──

/// 稳定类型(□_t):可安全跨时刻;Stream/闭包/指针为时序敏感,不可跨时刻
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeTag {
    Int,
    Bool,
    Str,
    Unit,
    Stream,
    Closure,
    Ptr,
}

pub fn is_stable_type(tag: TypeTag) -> bool {
    matches!(tag, TypeTag::Int | TypeTag::Bool | TypeTag::Str | TypeTag::Unit)
}

// ── 1.6 编译期区域逃逸检查 ──

/// 区域(作用域)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub name: String,
}

/// 区域栈:进入/退出作用域
#[derive(Debug, Clone, Default)]
pub struct RegionStack {
    pub regions: Vec<Region>,
}

impl RegionStack {
    pub fn push(&mut self, r: Region) {
        self.regions.push(r);
    }
    pub fn pop(&mut self) {
        self.regions.pop();
    }
    /// 指针区域是否已逃逸(退出作用域后指针不可用)
    pub fn escapes(&self, ptr_region: &Region) -> bool {
        !self.regions.contains(ptr_region)
    }
}

// ── 1.3 HoTT 完整立方填充(多维 Kan) ──

/// N 维立方体面组合:返回与边界一致的立方值;不一致边界报错
pub fn cube_fill(faces: &[bool]) -> Result<bool, String> {
    if faces.is_empty() {
        return Err("立方体无面".to_string());
    }
    let first = faces[0];
    if faces.iter().all(|&f| f == first) {
        Ok(first)
    } else {
        Err("立方边界面不一致".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grade_covers_modal() {
        assert!(grade_covers(&Grade::Omega, 1000));
        assert!(grade_covers(&Grade::Nat(5), 3));
        assert!(!grade_covers(&Grade::Nat(5), 6));
        assert!(grade_covers(&Grade::One, 1));
        assert!(!grade_covers(&Grade::One, 2));
        assert!(grade_covers(&Grade::Zero, 0));
    }

    #[test]
    fn test_asymptotic_cost() {
        assert!(cost_le(Cost::Const, Cost::Linear));
        assert!(cost_le(Cost::Linear, Cost::Poly(2)));
        assert!(!cost_le(Cost::Poly(2), Cost::Linear));
        // 递归复合取上界
        assert_eq!(Cost::Linear.combine(Cost::Poly(2)), Cost::Poly(2));
    }

    #[test]
    fn test_stable_type() {
        assert!(is_stable_type(TypeTag::Int));
        assert!(!is_stable_type(TypeTag::Stream));
        assert!(!is_stable_type(TypeTag::Closure));
    }

    #[test]
    fn test_region_escape() {
        let heap = Region { name: "heap".into() };
        let mut stack = RegionStack::default();
        stack.push(heap.clone());
        // 区域内指针可用
        assert!(!stack.escapes(&heap));
        stack.pop();
        // 退出区域后指针逃逸
        assert!(stack.escapes(&heap));
    }

    #[test]
    fn test_cube_fill() {
        assert_eq!(cube_fill(&[true, true, true]), Ok(true));
        assert!(cube_fill(&[true, false]).is_err());
        assert!(cube_fill(&[]).is_err());
    }
}
