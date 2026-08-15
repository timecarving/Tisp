use tisp_core::core_ast::*;
use tisp_core::symbol::Symbol;
use tisp_core::types::*;
use tisp_core::effects::*;
use tisp_core::span::Span;

#[derive(Debug, Clone)]
pub struct EffectError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for EffectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "effect error: {} at {}", self.message, self.span)
    }
}

impl std::error::Error for EffectError {}

/// §12.5:从效果行中移除被 handler 处理的效果标签
fn subtract_effects(row: &EffectRow, handled: &[EffectLabel]) -> EffectRow {
    match row {
        EffectRow::Pure => EffectRow::Pure,
        EffectRow::Closed(labels) => {
            let remaining: Vec<EffectLabel> = labels.iter()
                .filter(|l| !handled.contains(l))
                .cloned()
                .collect();
            if remaining.is_empty() {
                EffectRow::Pure
            } else {
                EffectRow::Closed(remaining)
            }
        }
        r => r.clone(),
    }
}

pub struct EffectInferrer {
    effect_env: EffectEnv,
}

impl EffectInferrer {
    pub fn new() -> Self {
        Self {
            effect_env: EffectEnv::new(),
        }
    }

    pub fn infer_program(&mut self, program: &CoreProgram) -> Result<Vec<(Symbol, EffectRow)>, EffectError> {
        let mut results = Vec::new();

        for def in &program.defs {
            let effects = self.infer_def(def)?;
            results.push((def.name.clone(), effects));
        }

        Ok(results)
    }

    fn infer_def(&mut self, def: &CoreDef) -> Result<EffectRow, EffectError> {
        self.infer_expr(&def.body)
    }

    fn infer_expr(&mut self, expr: &CoreExpr) -> Result<EffectRow, EffectError> {
        match &expr.node {
            CoreExprNode::Lit(_) => Ok(EffectRow::Pure),

            CoreExprNode::Var(_) => Ok(EffectRow::Pure),

            CoreExprNode::Lam(lambda) => {
                // Lambda itself is pure, but body may have effects
                self.infer_expr(&lambda.body)
            }

            CoreExprNode::App(func, arg) => {
                let func_eff = self.infer_expr(func)?;
                let arg_eff = self.infer_expr(arg)?;
                // §静态纯声明式:效果操作(如 println/put/throw)经内置调用时,须附着其效果,
                // 否则 (println ...) 会被误判为 Pure(违反纯声明式)
                let builtin_eff = match &func.node {
                    CoreExprNode::Var(name) => self.effect_env.lookup_operation(name)
                        .map(|e| EffectRow::Closed(vec![e]))
                        .unwrap_or(EffectRow::Pure),
                    _ => EffectRow::Pure,
                };
                Ok(row_union(&row_union(&func_eff, &builtin_eff), &arg_eff))
            }

            CoreExprNode::Let(_, _, value, body) => {
                let val_eff = self.infer_expr(value)?;
                let body_eff = self.infer_expr(body)?;
                Ok(row_union(&val_eff, &body_eff))
            }

            CoreExprNode::If(cond, then, else_) => {
                let cond_eff = self.infer_expr(cond)?;
                let then_eff = self.infer_expr(then)?;
                let else_eff = self.infer_expr(else_)?;
                Ok(row_union(&cond_eff, &row_union(&then_eff, &else_eff)))
            }

            CoreExprNode::Match(scrutinee, arms) => {
                let scrut_eff = self.infer_expr(scrutinee)?;
                let mut combined = scrut_eff;

                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        let guard_eff = self.infer_expr(guard)?;
                        combined = row_union(&combined, &guard_eff);
                    }
                    let body_eff = self.infer_expr(&arm.body)?;
                    combined = row_union(&combined, &body_eff);
                }

                Ok(combined)
            }

            CoreExprNode::Data(_, args) => {
                let mut combined = EffectRow::Pure;
                for arg in args {
                    let arg_eff = self.infer_expr(arg)?;
                    combined = row_union(&combined, &arg_eff);
                }
                Ok(combined)
            }

            CoreExprNode::Handle(body, handler) => {
                // Handler handles the effects in body
                // §12.5 行消减:移除被 handler clauses 处理的操作效果
                let body_eff = self.infer_expr(body)?;
                let handled: Vec<EffectLabel> = handler.clauses.iter()
                    .map(|c| self.effect_env.lookup_operation(&c.operation)
                        .unwrap_or_else(|| EffectLabel::Named(c.operation.clone())))
                    .collect();
                Ok(subtract_effects(&body_eff, &handled))
            }

            CoreExprNode::Perform(op_name, args) => {
                // Perform introduces an effect
                let mut arg_effs = EffectRow::Pure;
                for arg in args {
                    let arg_eff = self.infer_expr(arg)?;
                    arg_effs = row_union(&arg_effs, &arg_eff);
                }

                // Look up the operation's effect
                let op_eff = self.effect_env.lookup_operation(op_name)
                    .unwrap_or_else(|| EffectLabel::Named(op_name.clone()));

                let perform_eff = EffectRow::Closed(vec![op_eff]);
                Ok(row_union(&arg_effs, &perform_eff))
            }

            CoreExprNode::Hole(_) => Ok(EffectRow::Pure),

            CoreExprNode::Do(exprs) => {
                let mut combined = EffectRow::Pure;
                for e in exprs {
                    combined = row_union(&combined, &self.infer_expr(e)?);
                }
                Ok(combined)
            }
            // ── Concurrent operations (track effects) ──
            CoreExprNode::Spawn(e, _h) => {
                let body_eff = self.infer_expr(e)?;
                Ok(row_union(&body_eff, &EffectRow::Closed(vec![EffectLabel::Named(Symbol::new("Spawn"))])))
            }
            CoreExprNode::ChannelNew => Ok(EffectRow::Closed(vec![EffectLabel::Channel(Box::new(tisp_core::types::Type::i64()))])),
            CoreExprNode::ChannelSend(ch, val) => {
                let c = self.infer_expr(ch)?; let v = self.infer_expr(val)?;
                Ok(row_union(&row_union(&c, &v), &EffectRow::Closed(vec![EffectLabel::Channel(Box::new(tisp_core::types::Type::i64()))])))
            }
            CoreExprNode::ChannelRecv(e) => {
                let e_eff = self.infer_expr(e)?;
                Ok(row_union(&e_eff, &EffectRow::Closed(vec![EffectLabel::Channel(Box::new(tisp_core::types::Type::i64()))])))
            }
            CoreExprNode::AsyncSend(ch, val) => {
                let c = self.infer_expr(ch)?; let v = self.infer_expr(val)?;
                Ok(row_union(&row_union(&c, &v), &EffectRow::Closed(vec![EffectLabel::Named(Symbol::new("Async"))])))
            }
            CoreExprNode::AsyncRecv(e) => {
                let e_eff = self.infer_expr(e)?;
                Ok(row_union(&e_eff, &EffectRow::Closed(vec![EffectLabel::Named(Symbol::new("Async"))])))
            }
            CoreExprNode::Join(_h) => Ok(EffectRow::Closed(vec![EffectLabel::Named(Symbol::new("Spawn"))])),

            // ── Session type protocol tracking ──
            CoreExprNode::Session(op, operands) => {
                let session_eff = match op {
                    tisp_core::core_ast::SessionOp::Send => EffectRow::Closed(vec![EffectLabel::Session]),
                    tisp_core::core_ast::SessionOp::Recv => EffectRow::Closed(vec![EffectLabel::Session]),
                    tisp_core::core_ast::SessionOp::Close => EffectRow::Closed(vec![EffectLabel::Session]),
                    tisp_core::core_ast::SessionOp::Fork(_) => EffectRow::Closed(vec![EffectLabel::Session, EffectLabel::Named(Symbol::new("Spawn"))]),
                };
                let mut combined = session_eff.clone();
                for operand in operands {
                    combined = row_union(&self.infer_expr(operand)?, &combined);
                }
                Ok(combined)
            }

            // ── FRP Signal thread safety ──
            CoreExprNode::SignalNew(e) => {
                let e_eff = self.infer_expr(e)?;
                Ok(row_union(&e_eff, &EffectRow::Closed(vec![EffectLabel::Signal])))
            }
            CoreExprNode::SignalMap(s, _) | CoreExprNode::SignalFilter(s, _) => {
                let s_eff = self.infer_expr(s)?;
                Ok(row_union(&s_eff, &EffectRow::Closed(vec![EffectLabel::Signal])))
            }
            CoreExprNode::SignalFold(s, _, _) => {
                let s_eff = self.infer_expr(s)?;
                Ok(row_union(&s_eff, &EffectRow::Closed(vec![EffectLabel::Signal])))
            }
            CoreExprNode::SignalMerge(a, b) => {
                let a_eff = self.infer_expr(a)?; let b_eff = self.infer_expr(b)?;
                Ok(row_union(&row_union(&a_eff, &b_eff), &EffectRow::Closed(vec![EffectLabel::Signal])))
            }

            // §26/§7.5 裸内存原语:PtrRead/PtrWrite/RegionAlloc/RegionFree 引入 Unsafe
            CoreExprNode::PtrRead(e) => {
                let e_eff = self.infer_expr(e)?;
                Ok(row_union(&e_eff, &EffectRow::Closed(vec![EffectLabel::Named(Symbol::new("Unsafe"))])))
            }
            CoreExprNode::PtrWrite(a, b) => {
                let eff = row_union(&self.infer_expr(a)?, &self.infer_expr(b)?);
                Ok(row_union(&eff, &EffectRow::Closed(vec![EffectLabel::Named(Symbol::new("Unsafe"))])))
            }
            CoreExprNode::RegionAlloc(a, b) => {
                let eff = row_union(&self.infer_expr(a)?, &self.infer_expr(b)?);
                Ok(row_union(&eff, &EffectRow::Closed(vec![EffectLabel::Named(Symbol::new("Unsafe"))])))
            }
            CoreExprNode::RegionFree(e) => {
                let e_eff = self.infer_expr(e)?;
                Ok(row_union(&e_eff, &EffectRow::Closed(vec![EffectLabel::Named(Symbol::new("Unsafe"))])))
            }

            // §7.5 AOP 编织效应行合成:切面/方法体效应进入编织后定义
            CoreExprNode::MethodDef(_, _, _, body) | CoreExprNode::AdviceDef(_, _, _, body) => self.infer_expr(body),

            _ => Ok(EffectRow::Pure),
        }
    }
}

#[derive(Debug, Clone)]
struct EffectEnv {
    effects: Vec<EffectDecl>,
}

impl EffectEnv {
    fn new() -> Self {
        let mut env = Self { effects: Vec::new() };

        // Built-in effects
        env.register_effect(EffectDecl {
            name: Symbol::new("IO"),
            type_params: Vec::new(),
            operations: vec![
                OperationDecl {
                    name: Symbol::new("println"),
                    params: vec![Type::Con(TypeCon { name: Symbol::new("a"), kind: Kind::Star })],
                    return_type: Type::unit(),
                },
                OperationDecl {
                    name: Symbol::new("read-line"),
                    params: Vec::new(),
                    return_type: Type::string(),
                },
            ],
        });

        env.register_effect(EffectDecl {
            name: Symbol::new("State"),
            type_params: vec![Symbol::new("s")],
            operations: vec![
                OperationDecl {
                    name: Symbol::new("get"),
                    params: Vec::new(),
                    return_type: Type::Con(TypeCon { name: Symbol::new("s"), kind: Kind::Star }),
                },
                OperationDecl {
                    name: Symbol::new("put"),
                    params: vec![Type::Con(TypeCon { name: Symbol::new("s"), kind: Kind::Star })],
                    return_type: Type::unit(),
                },
            ],
        });

        env.register_effect(EffectDecl {
            name: Symbol::new("Except"),
            type_params: vec![Symbol::new("e")],
            operations: vec![
                OperationDecl {
                    name: Symbol::new("throw"),
                    params: vec![Type::Con(TypeCon { name: Symbol::new("e"), kind: Kind::Star })],
                    return_type: Type::Con(TypeCon { name: Symbol::new("a"), kind: Kind::Star }),
                },
            ],
        });

        // Unsafe:ptr-read/ptr-write/region-alloc/region-free(§26)
        env.register_effect(EffectDecl {
            name: Symbol::new("Unsafe"),
            type_params: Vec::new(),
            operations: vec![
                OperationDecl { name: Symbol::new("ptr-read"), params: vec![Type::i64()], return_type: Type::i64() },
                OperationDecl { name: Symbol::new("ptr-write"), params: vec![Type::i64(), Type::i64()], return_type: Type::unit() },
                OperationDecl { name: Symbol::new("region-alloc"), params: vec![Type::i64(), Type::i64()], return_type: Type::i64() },
                OperationDecl { name: Symbol::new("region-free"), params: vec![Type::i64()], return_type: Type::unit() },
            ],
        });

        // Search:choose(§21 回溯)+ ilp-induce(§31 归纳搜索)+ fresh(逻辑变量句柄)
        env.register_effect(EffectDecl {
            name: Symbol::new("Search"),
            type_params: Vec::new(),
            operations: vec![
                OperationDecl { name: Symbol::new("choose"), params: vec![Type::i64()], return_type: Type::i64() },
                OperationDecl { name: Symbol::new("ilp-induce"), params: vec![Type::list(Type::i64()), Type::list(Type::i64())], return_type: Type::list(Type::i64()) },
                OperationDecl { name: Symbol::new("fresh"), params: Vec::new(), return_type: Type::i64() },
            ],
        });

        // Signal:reactive-eval(§31 响应式信号派生)+ stream/stream-take(FRP 流句柄)
        env.register_effect(EffectDecl {
            name: Symbol::new("Signal"),
            type_params: Vec::new(),
            operations: vec![
                OperationDecl { name: Symbol::new("reactive-eval"), params: vec![Type::i64(), Type::i64()], return_type: Type::i64() },
                OperationDecl { name: Symbol::new("stream"), params: vec![Type::i64()], return_type: Type::i64() },
                OperationDecl { name: Symbol::new("stream-take"), params: vec![Type::i64(), Type::i64()], return_type: Type::list(Type::i64()) },
                OperationDecl { name: Symbol::new("stream-map"), params: vec![Type::i64(), Type::i64(), Type::i64()], return_type: Type::list(Type::i64()) },
                OperationDecl { name: Symbol::new("stream-filter"), params: vec![Type::i64(), Type::i64(), Type::i64()], return_type: Type::list(Type::i64()) },
                OperationDecl { name: Symbol::new("stream-sink"), params: vec![Type::i64(), Type::i64()], return_type: Type::list(Type::i64()) },
            ],
        });

        // Channel:send/recv(§27 π)+ chan(通道句柄创建)
        env.register_effect(EffectDecl {
            name: Symbol::new("Channel"),
            type_params: vec![Symbol::new("a")],
            operations: vec![
                OperationDecl { name: Symbol::new("chan"), params: Vec::new(), return_type: Type::string() },
                OperationDecl { name: Symbol::new("send"), params: vec![Type::string(), Type::i64()], return_type: Type::unit() },
                OperationDecl { name: Symbol::new("recv"), params: vec![Type::string()], return_type: Type::i64() },
            ],
        });

        // Reader:ask / Writer:tell
        env.register_effect(EffectDecl {
            name: Symbol::new("Reader"),
            type_params: vec![Symbol::new("r")],
            operations: vec![
                OperationDecl { name: Symbol::new("ask"), params: Vec::new(), return_type: Type::i64() },
            ],
        });
        env.register_effect(EffectDecl {
            name: Symbol::new("Writer"),
            type_params: vec![Symbol::new("w")],
            operations: vec![
                OperationDecl { name: Symbol::new("tell"), params: vec![Type::i64()], return_type: Type::unit() },
            ],
        });

        // State:补充 ref/deref/set!(§统一内存管理)
        env.register_effect(EffectDecl {
            name: Symbol::new("State"),
            type_params: vec![Symbol::new("s")],
            operations: vec![
                OperationDecl { name: Symbol::new("get"), params: Vec::new(), return_type: Type::i64() },
                OperationDecl { name: Symbol::new("put"), params: vec![Type::i64()], return_type: Type::unit() },
                OperationDecl { name: Symbol::new("ref"), params: vec![Type::i64()], return_type: Type::Ref(Box::new(Type::i64())) },
                OperationDecl { name: Symbol::new("deref"), params: vec![Type::Ref(Box::new(Type::i64()))], return_type: Type::i64() },
                OperationDecl { name: Symbol::new("set!"), params: vec![Type::Ref(Box::new(Type::i64())), Type::i64()], return_type: Type::unit() },
                OperationDecl { name: Symbol::new("get-kb"), params: Vec::new(), return_type: Type::i64() },
                OperationDecl { name: Symbol::new("set-kb"), params: vec![Type::list(Type::i64())], return_type: Type::unit() },
                OperationDecl { name: Symbol::new("stack-push"), params: vec![Type::i64(), Type::i64()], return_type: Type::i64() },
                OperationDecl { name: Symbol::new("stack-pop"), params: vec![Type::i64()], return_type: Type::i64() },
                OperationDecl { name: Symbol::new("stack-peek"), params: vec![Type::i64()], return_type: Type::i64() },
                OperationDecl { name: Symbol::new("stack-dup"), params: vec![Type::i64()], return_type: Type::i64() },
                OperationDecl { name: Symbol::new("stack-swap"), params: vec![Type::i64()], return_type: Type::i64() },
                OperationDecl { name: Symbol::new("stack-rotate"), params: vec![Type::i64(), Type::i64()], return_type: Type::i64() },
                OperationDecl { name: Symbol::new("stack-new"), params: Vec::new(), return_type: Type::i64() },
                OperationDecl { name: Symbol::new("sm-drive"), params: vec![Type::i64(), Type::i64(), Type::list(Type::i64())], return_type: Type::i64() },
                OperationDecl { name: Symbol::new("sm-new"), params: vec![Type::i64()], return_type: Type::i64() },
                OperationDecl { name: Symbol::new("table-new"), params: vec![Type::list(Type::i64()), Type::list(Type::i64())], return_type: Type::i64() },
                OperationDecl { name: Symbol::new("table-dispatch"), params: vec![Type::i64(), Type::i64(), Type::i64()], return_type: Type::i64() },
            ],
        });

        env
    }

    fn register_effect(&mut self, decl: EffectDecl) {
        self.effects.push(decl);
    }

    fn lookup_operation(&self, op_name: &Symbol) -> Option<EffectLabel> {
        for effect in &self.effects {
            for op in &effect.operations {
                if &op.name == op_name {
                    return Some(EffectLabel::Named(effect.name.clone()));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tisp_core::span::Span;
    use tisp_core::symbol::Symbol;

    fn e(node: CoreExprNode) -> CoreExpr {
        CoreExpr::new(node, Span::dummy())
    }

    #[test]
    fn test_println_is_io_not_pure() {
        // 静态纯声明式:println 是 IO 效应操作,调用后不得判为 Pure
        let call = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::Var(Symbol::new("println")))),
            Box::new(e(CoreExprNode::Lit(Literal::String("hello".into())))),
        ));
        let mut infer = EffectInferrer::new();
        let eff = infer.infer_expr(&call).unwrap();
        assert!(
            matches!(&eff, EffectRow::Closed(labels) if labels.iter().any(|l| matches!(l, EffectLabel::Named(n) if n.as_str() == "IO"))),
            "println 应引入 IO 效应,实际 {:?}", eff
        );
    }

    #[test]
    fn test_pure_literal_is_pure() {
        let lit = e(CoreExprNode::Lit(Literal::I64(42)));
        let mut infer = EffectInferrer::new();
        let eff = infer.infer_expr(&lit).unwrap();
        assert!(matches!(eff, EffectRow::Pure), "字面量应为 Pure,实际 {:?}", eff);
    }

    #[test]
    fn test_ptr_read_is_unsafe_not_pure() {
        // 静态纯声明式:ptr-read 是 Unsafe 效应操作,调用后不得判为 Pure
        let call = e(CoreExprNode::App(
            Box::new(e(CoreExprNode::Var(Symbol::new("ptr-read")))),
            Box::new(e(CoreExprNode::Lit(Literal::I64(0)))),
        ));
        let mut infer = EffectInferrer::new();
        let eff = infer.infer_expr(&call).unwrap();
        assert!(
            matches!(&eff, EffectRow::Closed(labels) if labels.iter().any(|l| matches!(l, EffectLabel::Named(n) if n.as_str() == "Unsafe"))),
            "ptr-read 应引入 Unsafe 效应,实际 {:?}", eff
        );
    }
}
