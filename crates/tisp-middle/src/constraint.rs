//! §2 统一六维约束求解:共享约束图。
//! 六维(type/effect/region/grade/mode/determinism)作为同一约束系统的投影,
//! 各 pass 产出的约束/错误在此统一记录,供协调器(solve.rs)按 fixpoint 收敛后
//! 统一报告跨维度上下文。

use std::collections::HashMap;
use tisp_core::span::Span;
use tisp_core::symbol::Symbol;

/// 约束维度(六维 + 类型族/特化等辅助维度)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dimension {
    Type,
    Effect,
    Region,
    Grade,
    Mode,
    Determinism,
}

impl std::fmt::Display for Dimension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Dimension::Type => "type",
            Dimension::Effect => "effect",
            Dimension::Region => "region",
            Dimension::Grade => "grade",
            Dimension::Mode => "mode",
            Dimension::Determinism => "determinism",
        };
        write!(f, "{}", s)
    }
}

/// 一条跨维度约束/冲突记录
#[derive(Debug, Clone)]
pub struct Constraint {
    pub dimension: Dimension,
    pub name: Option<Symbol>,
    pub message: String,
    pub span: Span,
}

/// 共享约束图:各 pass 产出的约束在此聚合
#[derive(Debug, Default)]
pub struct ConstraintGraph {
    constraints: Vec<Constraint>,
}

impl ConstraintGraph {
    pub fn new() -> Self {
        Self { constraints: Vec::new() }
    }

    /// 记录一条约束/冲突(带维度与 span);同维度同消息去重(fixpoint 迭代不重复累积)
    pub fn record(&mut self, dimension: Dimension, name: Option<Symbol>, message: impl Into<String>, span: Span) {
        let message = message.into();
        if !self.constraints.iter().any(|c| c.dimension == dimension && c.message == message) {
            self.constraints.push(Constraint { dimension, name, message, span });
        }
    }

    pub fn constraints(&self) -> &[Constraint] {
        &self.constraints
    }

    pub fn is_empty(&self) -> bool {
        self.constraints.is_empty()
    }

    /// 按维度分组(供跨维度上下文报告)
    pub fn by_dimension(&self) -> HashMap<Dimension, Vec<&Constraint>> {
        let mut m: HashMap<Dimension, Vec<&Constraint>> = HashMap::new();
        for c in &self.constraints {
            m.entry(c.dimension).or_default().push(c);
        }
        m
    }

    /// 合并另一个图
    pub fn merge(&mut self, other: ConstraintGraph) {
        self.constraints.extend(other.constraints);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constraint_graph_record_and_group() {
        let mut g = ConstraintGraph::new();
        g.record(Dimension::Type, Some(Symbol::new("f")), "type mismatch", Span::dummy());
        g.record(Dimension::Grade, Some(Symbol::new("f")), "grade violation", Span::dummy());
        assert!(!g.is_empty());
        assert_eq!(g.constraints().len(), 2);
        let by = g.by_dimension();
        assert_eq!(by.get(&Dimension::Type).map(|v| v.len()), Some(1));
        assert_eq!(by.get(&Dimension::Grade).map(|v| v.len()), Some(1));
    }
}
