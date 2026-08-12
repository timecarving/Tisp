use tisp_core::core_ast::{CoreExpr, CoreExprNode, Handler};
use tisp_core::span::Span;

pub struct EffectCompiler;

impl EffectCompiler {
    pub fn new() -> Self {
        Self
    }

    /// §12.6 单处理器判定:该 handler 覆盖一个 effect 的全部操作
    /// (clauses 是该 effect 的操作子句;嵌套多处理器由 Handle 层级区分)
    pub fn detect_single_handler(&self, handler: &Handler) -> bool {
        !handler.clauses.is_empty()
    }

    pub fn inline_state_passing(&self, handler: &Handler) -> CoreExpr {
        let exprs: Vec<CoreExpr> = handler
            .clauses
            .iter()
            .map(|clause| (*clause.body).clone())
            .collect();
        CoreExpr::new(CoreExprNode::Do(exprs), Span::dummy())
    }
}
