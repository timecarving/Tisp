use tisp_core::core_ast::*;
use tisp_core::symbol::Symbol;
use tisp_core::types::Mode;

use tisp_core::span::Span;

pub struct ModeAnalyzer {
    pub mode_env: std::collections::HashMap<Symbol, Mode>,
}

#[derive(Debug, Clone)]
pub struct ModeError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for ModeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mode error: {} at {}", self.message, self.span)
    }
}
impl std::error::Error for ModeError {}

impl ModeAnalyzer {
    pub fn new() -> Self {
        Self { mode_env: std::collections::HashMap::new() }
    }

    pub fn analyze_program(&mut self, program: &CoreProgram) -> Result<Vec<(Symbol, Mode)>, ModeError> {
        let mut results = Vec::new();
        for def in &program.defs {
            let mode = self.analyze_def(def)?;
            self.mode_env.insert(def.name.clone(), mode.clone());
            results.push((def.name.clone(), mode));
        }
        Ok(results)
    }

    fn analyze_def(&mut self, def: &CoreDef) -> Result<Mode, ModeError> {
        // For functions, infer mode from parameter usage
        let mode = def.mode.clone();
        Ok(mode)
    }

    /// Infer parameter modes from body expression
    pub fn infer_modes(&mut self, params: &[Param], body: &CoreExpr) -> Result<Vec<Mode>, ModeError> {
        let mut param_modes = Vec::new();
        for param in params {
            let mode = self.infer_mode_for_var(&param.name, body)?;
            param_modes.push(mode);
        }
        Ok(param_modes)
    }

    fn infer_mode_for_var(&self, var: &Symbol, expr: &CoreExpr) -> Result<Mode, ModeError> {
        let usage = self.count_usages(var, expr);
        let first_producer = self.find_first_binding(var, expr);
        match (usage, first_producer) {
            (0, _) => Ok(Mode::Free),      // Not used → output parameter
            (_, Some(pos)) if pos == 0 => Ok(Mode::Out),  // First occurrence is binding → producer
            _ => Ok(Mode::In),              // Used → input parameter
        }
    }

    /// Find the position where a variable is first bound (produced)
    fn find_first_binding(&self, var: &Symbol, expr: &CoreExpr) -> Option<usize> {
        self.find_binding_at(var, expr, 0)
    }

    fn find_binding_at(&self, var: &Symbol, expr: &CoreExpr, depth: usize) -> Option<usize> {
        match &expr.node {
            // Unification: (unify var value) — var is produced here
            CoreExprNode::Unify(a, b) => {
                if self.is_var_ref(a, var) && !self.is_var_ref(b, var) { return Some(depth); }
                if self.is_var_ref(b, var) && !self.is_var_ref(a, var) { return Some(depth); }
                None
            }
            // Let binding: (let [var value] body) — var is produced in the value
            CoreExprNode::Let(name, _, value, _) if name == var => Some(depth),
            // Do: look through sequential expressions
            CoreExprNode::Do(exprs) => {
                for e in exprs {
                    if let Some(d) = self.find_binding_at(var, e, depth) { return Some(d); }
                }
                None
            }
            // App: recurse into function and argument
            CoreExprNode::App(f, a) => {
                self.find_binding_at(var, f, depth).or_else(|| self.find_binding_at(var, a, depth))
            }
            _ => None,
        }
    }

    fn is_var_ref(&self, expr: &CoreExpr, var: &Symbol) -> bool {
        matches!(&expr.node, CoreExprNode::Var(name) if name == var)
    }

    fn count_usages(&self, var: &Symbol, expr: &CoreExpr) -> usize {
        match &expr.node {
            CoreExprNode::Var(name) if name == var => 1,
            CoreExprNode::Var(_) => 0,
            CoreExprNode::Lit(_) => 0,
            CoreExprNode::Hole(_) => 0,
            CoreExprNode::Do(exprs) => {
                exprs.iter().map(|e| self.count_usages(var, e)).sum()
            }
            CoreExprNode::Lam(lambda) => {
                if lambda.params.iter().any(|p| &p.name == var) {
                    0
                } else {
                    self.count_usages(var, &lambda.body)
                }
            }
            CoreExprNode::App(func, arg) => {
                self.count_usages(var, func) + self.count_usages(var, arg)
            }
            CoreExprNode::Let(_name, _, value, body) => {
                let val_usages = self.count_usages(var, value);
                val_usages + self.count_usages(var, body)
            }
            CoreExprNode::If(cond, then, else_) => {
                self.count_usages(var, cond)
                    + self.count_usages(var, then)
                    + self.count_usages(var, else_)
            }
            CoreExprNode::Match(scrutinee, arms) => {
                let mut count = self.count_usages(var, scrutinee);
                for arm in arms {
                    if !self.pattern_binds(var, &arm.pattern) {
                        count += self.count_usages(var, &arm.body);
                        if let Some(guard) = &arm.guard {
                            count += self.count_usages(var, guard);
                        }
                    }
                }
                count
            }
            CoreExprNode::Data(_, args) => {
                args.iter().map(|a| self.count_usages(var, a)).sum()
            }
            CoreExprNode::Handle(body, _handler) => {
                self.count_usages(var, body)
            }
            CoreExprNode::Perform(_, args) => {
                args.iter().map(|a| self.count_usages(var, a)).sum()
            }
            _ => 0,
        }
    }

    fn pattern_binds(&self, var: &Symbol, pat: &Pattern) -> bool {
        match pat {
            Pattern::Var(name) => name == var,
            Pattern::Wildcard => false,
            Pattern::Lit(_) => false,
            Pattern::Con(_, subpats) => subpats.iter().any(|p| self.pattern_binds(var, p)),
            Pattern::Tuple(pats) => pats.iter().any(|p| self.pattern_binds(var, p)),
        }
    }
}
