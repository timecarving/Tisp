use tisp_core::core_ast::{CoreExpr, CoreExprNode, Handler};
use tisp_core::span::Span;

pub struct EffectCompiler;

impl EffectCompiler {
    pub fn new() -> Self {
        Self
    }

    pub fn detect_single_handler(&self, handler: &Handler) -> bool {
        handler.clauses.len() == 1
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
