use tisp_core::core_ast::*;
use tisp_core::symbol::Symbol;
use tisp_core::types::Determinism;
use tisp_core::determinism::*;
use tisp_core::span::Span;

pub struct DeterminismAnalyzer {
    pub det_env: std::collections::HashMap<Symbol, Determinism>,
}

#[derive(Debug, Clone)]
pub struct DetError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for DetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "determinism error: {} at {}", self.message, self.span)
    }
}
impl std::error::Error for DetError {}

impl DeterminismAnalyzer {
    pub fn new() -> Self {
        Self { det_env: std::collections::HashMap::new() }
    }

    pub fn analyze_program(&mut self, program: &CoreProgram) -> Result<Vec<(Symbol, Determinism)>, DetError> {
        let mut results = Vec::new();
        for def in &program.defs {
            let det = self.analyze_def(def)?;
            self.det_env.insert(def.name.clone(), det.clone());
            results.push((def.name.clone(), det));
        }
        Ok(results)
    }

    fn analyze_def(&mut self, def: &CoreDef) -> Result<Determinism, DetError> {
        let cat = self.analyze_expr(&def.body)?;
        Ok(cat.to_det())
    }

    fn analyze_expr(&mut self, expr: &CoreExpr) -> Result<DetCategory, DetError> {
        match &expr.node {
            CoreExprNode::Lit(_) => {
                Ok(DetCategory { can_fail: false, max_solutions: MaxSolutions::One })
            }

            CoreExprNode::Var(name) => {
                if let Some(det) = self.det_env.get(name) {
                    Ok(DetCategory::from_det(det))
                } else {
                    Ok(DetCategory { can_fail: false, max_solutions: MaxSolutions::One })
                }
            }

            CoreExprNode::Lam(lambda) => {
                self.analyze_expr(&lambda.body)
            }

            CoreExprNode::App(func, arg) => {
                let f_cat = self.analyze_expr(func)?;
                let a_cat = self.analyze_expr(arg)?;
                Ok(det_conjunction(&f_cat, &a_cat))
            }

            CoreExprNode::Let(_, _, value, body) => {
                let v_cat = self.analyze_expr(value)?;
                let b_cat = self.analyze_expr(body)?;
                Ok(det_conjunction(&v_cat, &b_cat))
            }

            CoreExprNode::If(cond, then, else_) => {
                let c_cat = self.analyze_expr(cond)?;
                let t_cat = self.analyze_expr(then)?;
                let e_cat = self.analyze_expr(else_)?;
                let then_else = DetCategory {
                    can_fail: t_cat.can_fail || e_cat.can_fail,
                    max_solutions: match (t_cat.max_solutions, e_cat.max_solutions) {
                        (MaxSolutions::One, MaxSolutions::One) => MaxSolutions::One,
                        _ => MaxSolutions::Many,
                    },
                };
                Ok(det_conjunction(&c_cat, &then_else))
            }

            CoreExprNode::Match(scrutinee, arms) => {
                let scrut_cat = self.analyze_expr(scrutinee)?;
                let mut arms_cat = DetCategory { can_fail: true, max_solutions: MaxSolutions::Zero };

                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        let g_cat = self.analyze_expr(guard)?;
                        let b_cat = self.analyze_expr(&arm.body)?;
                        let arm_cat = det_conjunction(&g_cat, &b_cat);
                        arms_cat = DetCategory {
                            can_fail: arms_cat.can_fail && arm_cat.can_fail,
                            max_solutions: match (arms_cat.max_solutions, arm_cat.max_solutions) {
                                (MaxSolutions::Zero, x) | (x, MaxSolutions::Zero) => x,
                                (MaxSolutions::One, MaxSolutions::One) => MaxSolutions::One,
                                _ => MaxSolutions::Many,
                            },
                        };
                    } else {
                        let b_cat = self.analyze_expr(&arm.body)?;
                        arms_cat = DetCategory {
                            can_fail: arms_cat.can_fail && b_cat.can_fail,
                            max_solutions: match (arms_cat.max_solutions, b_cat.max_solutions) {
                                (MaxSolutions::Zero, x) | (x, MaxSolutions::Zero) => x,
                                (MaxSolutions::One, MaxSolutions::One) => MaxSolutions::One,
                                _ => MaxSolutions::Many,
                            },
                        };
                    }
                }

                Ok(det_conjunction(&scrut_cat, &arms_cat))
            }

            CoreExprNode::Data(_, args) => {
                let mut cat = DetCategory { can_fail: false, max_solutions: MaxSolutions::One };
                for arg in args {
                    cat = det_conjunction(&cat, &self.analyze_expr(arg)?);
                }
                Ok(cat)
            }

            CoreExprNode::Handle(body, _handler) => {
                self.analyze_expr(body)
            }

            CoreExprNode::Perform(_, args) => {
                let mut cat = DetCategory { can_fail: false, max_solutions: MaxSolutions::One };
                for arg in args {
                    cat = det_conjunction(&cat, &self.analyze_expr(arg)?);
                }
                Ok(cat)
            }

            CoreExprNode::Hole(_) => {
                Ok(DetCategory { can_fail: false, max_solutions: MaxSolutions::One })
            }

            CoreExprNode::Do(exprs) => {
                let mut cat = DetCategory { can_fail: false, max_solutions: MaxSolutions::One };
                for e in exprs { cat = self.analyze_expr(e)?; }
                Ok(cat)
            }
            _ => Ok(DetCategory { can_fail: false, max_solutions: MaxSolutions::One }),
        }
    }
}
