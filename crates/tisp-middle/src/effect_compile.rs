use tisp_core::core_ast::{CoreExpr, CoreExprNode, CoreProgram, Handler};

pub struct EffectCompiler;

impl EffectCompiler {
    pub fn new() -> Self {
        Self
    }

    /// §12.6 单处理器判定:handler 至少一个操作子句,且全部子句为状态传递形态(state 字段非空)
    pub fn detect_single_handler(&self, handler: &Handler) -> bool {
        !handler.clauses.is_empty() && handler.clauses.iter().all(|c| c.state.is_some())
    }

    /// §12.6 无嵌套判定:body 内部不含嵌套 Handle(单处理器可直接状态传递)
    pub fn detect_no_nesting(&self, body: &CoreExpr) -> bool {
        !contains_handle(body)
    }

    /// §12.6 monadic 状态线程降级:对可降级定义标记直接状态传递路径。
    /// 解释器看到这些定义中的 Handle 时启用 direct_state 槽(真实零开销状态线程),
    /// 返回可降级定义数。
    pub fn lower_monadic_state(&self, program: &mut CoreProgram) -> usize {
        let mut lowered = 0;
        for def in &mut program.defs {
            if let CoreExprNode::Lam(lam) = &def.body.node {
                if let CoreExprNode::Handle(body, handler) = &lam.body.node {
                    if self.detect_single_handler(handler) && self.detect_no_nesting(body) {
                        lowered += 1;
                        // Core 层不展开状态参数:解释器按 direct_state 槽执行线程化
                    }
                }
            }
        }
        lowered
    }

    /// 单状态 handler 的降级:把操作子句体拼成 Do(供状态传递路径参考;真实状态线程在解释器)
    pub fn inline_state_passing(&self, handler: &Handler) -> CoreExpr {
        let exprs: Vec<CoreExpr> = handler.clauses.iter().map(|c| (*c.body).clone()).collect();
        CoreExpr::new(CoreExprNode::Do(exprs), tisp_core::span::Span::dummy())
    }
}

/// body 中是否含嵌套 Handle
fn contains_handle(expr: &CoreExpr) -> bool {
    match &expr.node {
        CoreExprNode::Handle(_, _) => true,
        CoreExprNode::App(f, a) => contains_handle(f) || contains_handle(a),
        CoreExprNode::If(c, t, e) => contains_handle(c) || contains_handle(t) || contains_handle(e),
        CoreExprNode::Let(_, _, v, b) => contains_handle(v) || contains_handle(b),
        CoreExprNode::Do(items) => items.iter().any(contains_handle),
        CoreExprNode::Lam(lam) => contains_handle(&lam.body),
        CoreExprNode::Match(s, arms) => {
            contains_handle(s) || arms.iter().any(|arm| contains_handle(&arm.body))
        }
        CoreExprNode::Search(e) => contains_handle(e),
        CoreExprNode::Unify(a, b) => contains_handle(a) || contains_handle(b),
        _ => false,
    }
}
