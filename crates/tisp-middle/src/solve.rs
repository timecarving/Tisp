//! §2 统一六维约束求解:协调器。
//! 把六个独立 pass(type/effect/region/grade/mode/determinism)收敛到共享约束图,
//! 按固定序运行并聚合各维度约束/冲突,统一报告跨维度上下文(替换「先到先报错」)。
//!
//! 说明:完整 fixpoint 迭代(维度间相互约束、迭代至收敛)是设计目标;当前实现
//! 为增量收敛第一步——串行运行各 pass 并聚合冲突,不改变各 pass 的既有语义。

use tisp_core::core_ast::CoreProgram;
use crate::constraint::{ConstraintGraph, Dimension};

/// 统一约束求解器
pub struct ConstraintSolver;

impl ConstraintSolver {
    pub fn new() -> Self {
        Self
    }

    /// 求解六维约束:运行各 pass,把冲突聚合进共享约束图。
    /// 返回 (约束图, 是否无冲突)。
    pub fn solve(&mut self, program: &CoreProgram) -> (ConstraintGraph, bool) {
        let mut graph = ConstraintGraph::new();

        // Type
        let mut type_infer = crate::type_infer::TypeInfer::new();
        if let Err(e) = type_infer.infer_program(program) {
            graph.record(Dimension::Type, None, e.message, e.span);
        }

        // Effect
        let mut effect_infer = crate::effect_infer::EffectInferrer::new();
        if let Err(e) = effect_infer.infer_program(program) {
            graph.record(Dimension::Effect, None, e.message, e.span);
        }

        // Grade
        let mut grade_checker = crate::grade_check::GradeChecker::new();
        if let Err(e) = grade_checker.check_program(program) {
            graph.record(Dimension::Grade, None, e.message, e.span);
        }

        // Determinism
        let mut det_analyzer = crate::determinism_analysis::DeterminismAnalyzer::new();
        if let Err(e) = det_analyzer.analyze_program(program) {
            graph.record(Dimension::Determinism, None, e.message, e.span);
        }

        // Mode
        let mut mode_analyzer = crate::mode_analysis::ModeAnalyzer::new();
        if let Err(e) = mode_analyzer.analyze_program(program) {
            graph.record(Dimension::Mode, None, e.message, e.span);
        }

        // Region
        let mut region_infer = crate::region_infer::RegionInfer::new();
        if let Err(e) = region_infer.infer_program(program) {
            graph.record(Dimension::Region, None, e.message, e.span);
        }

        let clean = graph.is_empty();
        (graph, clean)
    }
}

/// 把约束图格式化为跨维度上下文的诊断报告
pub fn format_report(graph: &ConstraintGraph) -> String {
    if graph.is_empty() {
        return String::new();
    }
    let mut out = String::from("约束求解冲突(跨维度上下文):\n");
    for c in graph.constraints() {
        out.push_str(&format!("  [{}] {} (span {})\n", c.dimension, c.message, c.span));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tisp_core::core_ast::*;
    use tisp_core::symbol::Symbol;
    use tisp_core::span::Span;
    use tisp_core::types::{EffectRow, Grade, Mode, Determinism};

    fn e(node: CoreExprNode) -> CoreExpr {
        CoreExpr::new(node, Span::dummy())
    }

    /// 构造一个含类型错误的程序:把 i64 当函数调用
    fn bad_program() -> CoreProgram {
        let def = CoreDef {
            name: Symbol::new("f"),
            ty: None,
            effects: EffectRow::Pure,
            grade: Grade::Omega,
            mode: Mode::In,
            region: None,
            visibility: Visibility::Public,
            mode_sigs: vec![],
            determinism: Determinism::Det,
            body: e(CoreExprNode::Lam(Lambda {
                params: vec![],
                body: Box::new(e(CoreExprNode::App(
                    Box::new(e(CoreExprNode::Lit(Literal::I64(1)))),
                    Box::new(e(CoreExprNode::Lit(Literal::I64(2)))),
                ))),
                ret_type: None,
            })),
            requires: None,
            ensures: None,
            span: Span::dummy(),
        };
        CoreProgram {
            data_decls: vec![],
            effect_decls: vec![],
            type_families: vec![],
            resource_algebras: vec![],
            defs: vec![def],
            pragmas: vec![],
        }
    }

    #[test]
    fn test_solver_reports_cross_dimension() {
        let mut solver = ConstraintSolver::new();
        let (graph, clean) = solver.solve(&bad_program());
        assert!(!clean, "类型错误程序应有冲突");
        assert!(!graph.is_empty());
        let report = format_report(&graph);
        assert!(report.contains("[type]"), "报告应含 type 维度,实际: {}", report);
    }

    #[test]
    fn test_solver_clean_program() {
        // 无错误的程序:字面量
        let def = CoreDef {
            name: Symbol::new("f"),
            ty: None, effects: EffectRow::Pure, grade: Grade::Omega, mode: Mode::In,
            region: None, visibility: Visibility::Public, mode_sigs: vec![], determinism: Determinism::Det,
            body: e(CoreExprNode::Lam(Lambda {
                params: vec![], body: Box::new(e(CoreExprNode::Lit(Literal::I64(42)))), ret_type: None,
            })),
            requires: None, ensures: None, span: Span::dummy(),
        };
        let program = CoreProgram {
            data_decls: vec![], effect_decls: vec![], type_families: vec![],
            resource_algebras: vec![], defs: vec![def], pragmas: vec![],
        };
        let mut solver = ConstraintSolver::new();
        let (graph, clean) = solver.solve(&program);
        assert!(clean, "正确程序应无冲突");
        assert!(graph.is_empty());
    }
}
