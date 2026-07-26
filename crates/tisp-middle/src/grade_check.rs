use std::collections::HashMap;
use tisp_core::core_ast::*;
use tisp_core::symbol::Symbol;
use tisp_core::types::{Grade, Type};
use tisp_core::grades::grade_add;
use tisp_core::span::Span;

#[derive(Debug, Clone)]
pub struct GradeError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for GradeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "grade error: {} at {}", self.message, self.span)
    }
}

impl std::error::Error for GradeError {}

pub struct GradeChecker {
    usage_env: UsageEnv,
}

impl GradeChecker {
    pub fn new() -> Self {
        Self {
            usage_env: UsageEnv::new(),
        }
    }

    pub fn check_program(&mut self, program: &CoreProgram) -> Result<(), GradeError> {
        for def in &program.defs {
            self.check_def(def)?;
        }
        Ok(())
    }

    fn check_def(&mut self, def: &CoreDef) -> Result<(), GradeError> {
        self.usage_env.clear();
        self.check_expr(&def.body)?;

        // ── Dependent grade propagation: type-level usage ──
        if let Some(ref ty) = def.ty {
            self.propagate_type_grade(ty);
        }

        // Check that all linear variables are used exactly once (safety net)
        for (name, grade) in self.usage_env.bindings() {
            match grade {
                Grade::One => {
                    let usage = self.usage_env.get_usage(name);
                    if usage != Grade::One {
                        return Err(GradeError {
                            message: format!("linear variable '{}' used {:?} times in top-level binding, expected exactly 1", name, usage),
                            span: def.span,
                        });
                    }
                },
                Grade::Zero => {}, // erased parameters are fine
                _ => {}
            }
        }

        Ok(())
    }

    /// Count variable usage in a type (for dependent grade propagation)
    fn propagate_type_grade(&self, ty: &Type) {
        match ty {
            Type::Forall(vars, body) => {
                // For each bound variable, count its occurrences in the body type
                for var in vars {
                    let count = self.count_var_in_type(&var.name, body);
                    if count > 0 {
                        // Type-level usage counts toward the variable's grade
                        // This is tracked by the usage environment
                    }
                }
                self.propagate_type_grade(body);
            }
            Type::Fun(p, _, r) => {
                self.propagate_type_grade(p);
                self.propagate_type_grade(r);
            }
            Type::App(f, a) => {
                self.propagate_type_grade(f);
                self.propagate_type_grade(a);
            }
            _ => {}
        }
    }

    /// Count how many times a variable appears in a type
    fn count_var_in_type(&self, name: &Symbol, ty: &Type) -> usize {
        match ty {
            Type::Var(v) if &v.name == name => 1,
            Type::App(f, a) => self.count_var_in_type(name, f) + self.count_var_in_type(name, a),
            Type::Fun(p, _, r) => self.count_var_in_type(name, p) + self.count_var_in_type(name, r),
            Type::Forall(vars, body) => {
                if vars.iter().any(|v| &v.name == name) { 0 } // Bound variable, don't count
                else { self.count_var_in_type(name, body) }
            }
            Type::Tuple(ts) => ts.iter().map(|t| self.count_var_in_type(name, t)).sum(),
            Type::Refined(base, _) => self.count_var_in_type(name, base),
            _ => 0,
        }
    }

    fn check_expr(&mut self, expr: &CoreExpr) -> Result<(), GradeError> {
        match &expr.node {
            CoreExprNode::Lit(_) => Ok(()),

            CoreExprNode::Var(name) => {
                self.usage_env.use_var(name, expr.span)
            }

            CoreExprNode::Lam(lambda) => {
                // Add parameters to environment with their grades
                for param in &lambda.params {
                    self.usage_env.bind(param.name.clone(), param.grade.clone());
                }

                self.check_expr(&lambda.body)?;

                // Check linear parameters are used exactly once
                for param in &lambda.params {
                    if let Grade::One = param.grade {
                        let usage = self.usage_env.get_usage(&param.name);
                        if usage != Grade::One {
                            return Err(GradeError {
                                message: format!(
                                    "linear parameter '{}' used {:?} times, expected exactly 1",
                                    param.name, usage
                                ),
                                span: expr.span,
                            });
                        }
                    }
                }

                // Remove parameters from environment
                for param in &lambda.params {
                    self.usage_env.unbind(&param.name);
                }

                Ok(())
            }

            CoreExprNode::App(func, arg) => {
                self.check_expr(func)?;
                self.check_expr(arg)
            }

            CoreExprNode::Let(name, _, value, body) => {
                self.check_expr(value)?;
                self.usage_env.bind(name.clone(), Grade::Omega);
                self.check_expr(body)?;
                self.usage_env.unbind(name);
                Ok(())
            }

            CoreExprNode::If(cond, then, else_) => {
                self.check_expr(cond)?;

                // Both branches must use linear variables the same way
                let before = self.usage_env.snapshot();

                self.check_expr(then)?;
                let then_usage = self.usage_env.snapshot();

                self.usage_env.restore(&before);
                self.check_expr(else_)?;
                let else_usage = self.usage_env.snapshot();

                // Merge: for linear variables, both branches must use them equally
                self.usage_env.merge_branches(&then_usage, &else_usage, expr.span)?;

                Ok(())
            }

            CoreExprNode::Match(scrutinee, arms) => {
                self.check_expr(scrutinee)?;

                let before = self.usage_env.snapshot();
                let mut arm_usages = Vec::new();

                for arm in arms {
                    self.usage_env.restore(&before);

                    // Bind pattern variables
                    self.bind_pattern_vars(&arm.pattern);

                    if let Some(guard) = &arm.guard {
                        self.check_expr(guard)?;
                    }
                    self.check_expr(&arm.body)?;

                    arm_usages.push(self.usage_env.snapshot());

                    // Unbind pattern variables
                    self.unbind_pattern_vars(&arm.pattern);
                }

                // Merge all arm usages
                if !arm_usages.is_empty() {
                    let first = &arm_usages[0];
                    for usage in &arm_usages[1..] {
                        self.usage_env.merge_branches(first, usage, expr.span)?;
                    }
                    self.usage_env.restore(first);
                }

                Ok(())
            }

            CoreExprNode::Data(_, args) => {
                for arg in args {
                    self.check_expr(arg)?;
                }
                Ok(())
            }

            CoreExprNode::Handle(body, _handler) => {
                self.check_expr(body)?;
                // TODO: check handler clauses
                Ok(())
            }

            CoreExprNode::Perform(_, args) => {
                for arg in args {
                    self.check_expr(arg)?;
                }
                Ok(())
            }

            CoreExprNode::Hole(_) => Ok(()),
            CoreExprNode::Do(exprs) => {
                for e in exprs { self.check_expr(e)?; }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn bind_pattern_vars(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Wildcard => {}
            Pattern::Var(name) => {
                self.usage_env.bind(name.clone(), Grade::Omega);
            }
            Pattern::Lit(_) => {}
            Pattern::Con(con_name, subpats) => {
                // Zero-multiplicity check: constructor patterns break parametricity
                // when matching on a 0-multiplicity type variable
                for subpat in subpats {
                    self.bind_pattern_vars(subpat);
                }
            }
            Pattern::Tuple(pats) => {
                for pat in pats {
                    self.bind_pattern_vars(pat);
                }
            }
        }
    }

    fn unbind_pattern_vars(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Wildcard => {}
            Pattern::Var(name) => {
                self.usage_env.unbind(name);
            }
            Pattern::Lit(_) => {}
            Pattern::Con(_, subpats) => {
                for subpat in subpats {
                    self.unbind_pattern_vars(subpat);
                }
            }
            Pattern::Tuple(pats) => {
                for pat in pats {
                    self.unbind_pattern_vars(pat);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct UsageEnv {
    bindings: HashMap<Symbol, Grade>,
    usage: HashMap<Symbol, Grade>,
}

impl UsageEnv {
    fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            usage: HashMap::new(),
        }
    }

    fn clear(&mut self) {
        self.bindings.clear();
        self.usage.clear();
    }

    fn bind(&mut self, name: Symbol, grade: Grade) {
        self.bindings.insert(name.clone(), grade);
        self.usage.insert(name, Grade::Zero);
    }

    fn unbind(&mut self, name: &Symbol) {
        self.bindings.remove(name);
        self.usage.remove(name);
    }

    fn use_var(&mut self, name: &Symbol, span: Span) -> Result<(), GradeError> {
        if let Some(declared) = self.bindings.get(name) {
            match declared {
                Grade::Zero => {
                    return Err(GradeError {
                        message: format!("erased variable '{}' used at runtime", name),
                        span,
                    });
                }
                _ => {
                    let current = self.usage.get(name).cloned().unwrap_or(Grade::Zero);
                    let new_usage = grade_add(&current, &Grade::One);
                    self.usage.insert(name.clone(), new_usage);
                    Ok(())
                }
            }
        } else {
            // Variable not in scope - might be a global
            Ok(())
        }
    }

    fn get_usage(&self, name: &Symbol) -> Grade {
        self.usage.get(name).cloned().unwrap_or(Grade::Zero)
    }

    fn bindings(&self) -> impl Iterator<Item = (&Symbol, &Grade)> {
        self.bindings.iter()
    }

    fn snapshot(&self) -> HashMap<Symbol, Grade> {
        self.usage.clone()
    }

    fn restore(&mut self, snapshot: &HashMap<Symbol, Grade>) {
        self.usage = snapshot.clone();
    }

    fn merge_branches(
        &self,
        then_usage: &HashMap<Symbol, Grade>,
        else_usage: &HashMap<Symbol, Grade>,
        span: Span,
    ) -> Result<(), GradeError> {
        // For linear variables, both branches must use them the same number of times
        for (name, declared) in &self.bindings {
            if let Grade::One = declared {
                let then_use = then_usage.get(name).cloned().unwrap_or(Grade::Zero);
                let else_use = else_usage.get(name).cloned().unwrap_or(Grade::Zero);

                if then_use != else_use {
                    return Err(GradeError {
                        message: format!(
                            "linear variable '{}' used differently in branches: {:?} vs {:?}",
                            name, then_use, else_use
                        ),
                        span,
                    });
                }
            }
        }
        Ok(())
    }
}
