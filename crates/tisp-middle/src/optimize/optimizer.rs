use tisp_core::core_ast::*;
use tisp_core::symbol::Symbol;

use tisp_core::span::Span;
use std::collections::HashMap;

/// Optimization pass: inlining, constant folding, dead code elimination, strictness
pub struct Optimizer {
    /// Function definitions available for inlining
    inline_candidates: HashMap<Symbol, CoreDef>,
    /// §30 opt-level 控制的内联阈值(expr_size ≤ threshold 才内联)
    inline_threshold: usize,
    /// §30 inline! 强制内联的目标函数名
    force_inline: Vec<Symbol>,
    /// Optimization statistics
    pub stats: OptStats,
}

#[derive(Debug, Default, Clone)]
pub struct OptStats {
    pub inlined: usize,
    pub folded: usize,
    pub dead_eliminated: usize,
    pub strictness_markers: usize,
}

impl Optimizer {
    pub fn new() -> Self {
        Self {
            inline_candidates: HashMap::new(),
            inline_threshold: 5,
            force_inline: Vec::new(),
            stats: OptStats::default(),
        }
    }

    /// §30 编译指示接线:opt-level 调内联阈值;inline! 强制内联;noinline! 禁止内联
    pub fn configure(&mut self, pragmas: &[(Symbol, Vec<Symbol>)]) {
        for (name, targets) in pragmas {
            match name.as_str() {
                "opt-level" => {
                    // opt-level N:提高内联阈值(N 越大越激进);level 0 关闭内联
                    if let Some(t) = targets.first() {
                        if let Ok(n) = t.as_str().parse::<u64>() {
                            self.inline_threshold = if n == 0 { 0 } else { 5 + (n as usize) * 3 };
                        }
                    }
                }
                "inline!" => {
                    for t in targets { self.force_inline.push(t.clone()); }
                }
                "noinline!" => {
                    // noinline!:从内联候选中移除
                    for t in targets { self.inline_candidates.remove(t); }
                }
                _ => {}
            }
        }
    }

    /// Optimize a CoreProgram — returns optimized version
    pub fn optimize(&mut self, program: &CoreProgram) -> CoreProgram {
        // Register all definitions as inline candidates
        for def in &program.defs {
            self.inline_candidates.insert(def.name.clone(), def.clone());
        }

        let mut optimized_defs = Vec::new();
        for def in &program.defs {
            let opt_body = self.optimize_expr(&def.body);
            let mut opt_def = def.clone();
            opt_def.body = opt_body;
            optimized_defs.push(opt_def);
        }

        // Dead code elimination: remove unused definitions
        let used_names = self.collect_used_names(&optimized_defs);
        optimized_defs.retain(|d| used_names.contains(&d.name) || d.name.as_str() == "main");

        CoreProgram {
            data_decls: program.data_decls.clone(),
            type_families: program.type_families.clone(),
            resource_algebras: program.resource_algebras.clone(),
            effect_decls: program.effect_decls.clone(),
            defs: optimized_defs,
            pragmas: vec![],
        }
    }

    fn optimize_expr(&mut self, expr: &CoreExpr) -> CoreExpr {
        let optimized = match &expr.node {
            CoreExprNode::App(func, arg) => {
                let opt_func = self.optimize_expr(func);
                let opt_arg = self.optimize_expr(arg);

                // Try inlining: if the function is a known definition and it's small
                if let Some(inlined) = self.try_inline(&opt_func, &opt_arg, expr.span) {
                    self.stats.inlined += 1;
                    inlined
                } else {
                    CoreExpr::new(
                        CoreExprNode::App(Box::new(opt_func), Box::new(opt_arg)),
                        expr.span,
                    )
                }
            }

            CoreExprNode::Lam(lambda) => {
                let opt_body = self.optimize_expr(&lambda.body);
                CoreExpr::new(
                    CoreExprNode::Lam(Lambda {
                        params: lambda.params.clone(),
                        body: Box::new(opt_body),
                        ret_type: lambda.ret_type.clone(),
                    }),
                    expr.span,
                )
            }

            CoreExprNode::Let(name, ty, value, body) => {
                let opt_value = self.optimize_expr(value);
                let opt_body = self.optimize_expr(body);

                // Dead code elimination: if name is unused in body, drop the let
                if !self.uses_var(name, &opt_body) && !has_side_effects(&opt_value) {
                    self.stats.dead_eliminated += 1;
                    opt_body
                } else {
                    CoreExpr::new(
                        CoreExprNode::Let(name.clone(), ty.clone(), Box::new(opt_value), Box::new(opt_body)),
                        expr.span,
                    )
                }
            }

            CoreExprNode::If(cond, then, else_) => {
                let opt_cond = self.optimize_expr(cond);
                let opt_then = self.optimize_expr(then);
                let opt_else = self.optimize_expr(else_);

                // Constant folding: if condition is constant
                if let CoreExprNode::Lit(Literal::Bool(b)) = &opt_cond.node {
                    self.stats.folded += 1;
                    if *b { opt_then } else { opt_else }
                } else {
                    CoreExpr::new(
                        CoreExprNode::If(Box::new(opt_cond), Box::new(opt_then), Box::new(opt_else)),
                        expr.span,
                    )
                }
            }

            CoreExprNode::Match(scrutinee, arms) => {
                let opt_scrutinee = self.optimize_expr(scrutinee);
                let opt_arms: Vec<_> = arms.iter().map(|arm| {
                    let body = self.optimize_expr(&arm.body);
                    let guard = arm.guard.as_ref().map(|g| self.optimize_expr(g));
                    MatchArm {
                        pattern: arm.pattern.clone(),
                        guard: guard.map(Box::new),
                        body: Box::new(body),
                    }
                }).collect();
                CoreExpr::new(
                    CoreExprNode::Match(Box::new(opt_scrutinee), opt_arms),
                    expr.span,
                )
            }

            CoreExprNode::Data(name, args) => {
                let opt_args: Vec<_> = args.iter().map(|a| self.optimize_expr(a)).collect();
                CoreExpr::new(
                    CoreExprNode::Data(name.clone(), opt_args),
                    expr.span,
                )
            }

            CoreExprNode::Handle(body, handler) => {
                let opt_body = self.optimize_expr(body);
                CoreExpr::new(
                    CoreExprNode::Handle(Box::new(opt_body), handler.clone()),
                    expr.span,
                )
            }

            CoreExprNode::Perform(name, args) => {
                let opt_args: Vec<_> = args.iter().map(|a| self.optimize_expr(a)).collect();
                CoreExpr::new(
                    CoreExprNode::Perform(name.clone(), opt_args),
                    expr.span,
                )
            }

            CoreExprNode::Lit(_) | CoreExprNode::Var(_) | CoreExprNode::Hole(_) => {
                expr.clone()
            }

            CoreExprNode::Do(exprs) => {
                let opt: Vec<_> = exprs.iter().map(|e| self.optimize_expr(e)).collect();
                CoreExpr::new(CoreExprNode::Do(opt), expr.span)
            }

            _ => expr.clone(),
        };

        // Constant folding for arithmetic: (+ 1 2) → 3
        self.try_constant_fold(&optimized)
    }

    /// Try to inline a function call
    fn try_inline(&self, func: &CoreExpr, arg: &CoreExpr, _span: Span) -> Option<CoreExpr> {
        if let CoreExprNode::Var(name) = &func.node {
            if let Some(def) = self.inline_candidates.get(name) {
                // §30 inline!:强制内联(无视阈值);否则按 opt-level 阈值判断
                let force = self.force_inline.iter().any(|f| f == name);
                if force || self.is_small_body(&def.body) {
                    if let CoreExprNode::Lam(lambda) = &def.body.node {
                        if lambda.params.len() == 1 {
                            let param = &lambda.params[0];
                            // Substitute the argument for the parameter
                            let inlined = self.substitute_var(&lambda.body, &param.name, arg);
                            return Some(inlined);
                        }
                    }
                }
            }
        }
        None
    }

    /// Check if a body is small enough to inline
    fn is_small_body(&self, expr: &CoreExpr) -> bool {
        self.expr_size(expr) <= self.inline_threshold
    }

    fn expr_size(&self, expr: &CoreExpr) -> usize {
        match &expr.node {
            CoreExprNode::Lit(_) | CoreExprNode::Var(_) | CoreExprNode::Hole(_) => 1,
            CoreExprNode::App(f, a) => 1 + self.expr_size(f) + self.expr_size(a),
            CoreExprNode::Lam(l) => 1 + self.expr_size(&l.body),
            CoreExprNode::Let(_, _, v, b) => 1 + self.expr_size(v) + self.expr_size(b),
            CoreExprNode::If(c, t, e) => 1 + self.expr_size(c) + self.expr_size(t) + self.expr_size(e),
            CoreExprNode::Match(s, arms) => {
                1 + self.expr_size(s) + arms.iter().map(|a| self.expr_size(&a.body)).sum::<usize>()
            }
            CoreExprNode::Data(_, args) => 1 + args.iter().map(|a| self.expr_size(a)).sum::<usize>(),
            CoreExprNode::Handle(b, _) => 1 + self.expr_size(b),
            CoreExprNode::Perform(_, args) => 1 + args.iter().map(|a| self.expr_size(a)).sum::<usize>(),
            CoreExprNode::Do(exprs) => exprs.iter().map(|e| self.expr_size(e)).sum::<usize>(),
            _ => 1,
        }
    }

    /// Substitute a variable with an expression in a body
    fn substitute_var(&self, expr: &CoreExpr, var: &Symbol, replacement: &CoreExpr) -> CoreExpr {
        match &expr.node {
            CoreExprNode::Var(name) if name == var => replacement.clone(),
            CoreExprNode::Lam(lambda) => {
                // Don't substitute inside if the parameter shadows the variable
                if lambda.params.iter().any(|p| &p.name == var) {
                    expr.clone()
                } else {
                    CoreExpr::new(
                        CoreExprNode::Lam(Lambda {
                            params: lambda.params.clone(),
                            body: Box::new(self.substitute_var(&lambda.body, var, replacement)),
                            ret_type: lambda.ret_type.clone(),
                        }),
                        expr.span,
                    )
                }
            }
            CoreExprNode::App(f, a) => CoreExpr::new(
                CoreExprNode::App(
                    Box::new(self.substitute_var(f, var, replacement)),
                    Box::new(self.substitute_var(a, var, replacement)),
                ),
                expr.span,
            ),
            CoreExprNode::Let(name, ty, v, b) => {
                let new_v = self.substitute_var(v, var, replacement);
                let new_b = if name == var { b.as_ref().clone() } else { self.substitute_var(b, var, replacement) };
                CoreExpr::new(CoreExprNode::Let(name.clone(), ty.clone(), Box::new(new_v), Box::new(new_b)), expr.span)
            }
            CoreExprNode::If(c, t, e) => CoreExpr::new(
                CoreExprNode::If(
                    Box::new(self.substitute_var(c, var, replacement)),
                    Box::new(self.substitute_var(t, var, replacement)),
                    Box::new(self.substitute_var(e, var, replacement)),
                ),
                expr.span,
            ),
            CoreExprNode::Match(s, arms) => {
                let new_s = self.substitute_var(s, var, replacement);
                let new_arms = arms.iter().map(|arm| {
                    // Don't substitute if the pattern binds the variable
                    if self.pattern_binds_var(&arm.pattern, var) {
                        arm.clone()
                    } else {
                        MatchArm {
                            pattern: arm.pattern.clone(),
                            guard: arm.guard.as_ref().map(|g| Box::new(self.substitute_var(g, var, replacement))),
                            body: Box::new(self.substitute_var(&arm.body, var, replacement)),
                        }
                    }
                }).collect();
                CoreExpr::new(CoreExprNode::Match(Box::new(new_s), new_arms), expr.span)
            }
            CoreExprNode::Data(name, args) => {
                let new_args = args.iter().map(|a| self.substitute_var(a, var, replacement)).collect();
                CoreExpr::new(CoreExprNode::Data(name.clone(), new_args), expr.span)
            }
            _ => expr.clone(),
        }
    }

    fn pattern_binds_var(&self, pat: &Pattern, var: &Symbol) -> bool {
        match pat {
            Pattern::Var(name) => name == var,
            Pattern::Con(_, subpats) => subpats.iter().any(|p| self.pattern_binds_var(p, var)),
            Pattern::Tuple(pats) => pats.iter().any(|p| self.pattern_binds_var(p, var)),
            _ => false,
        }
    }

    /// Try constant folding on arithmetic expressions
    fn try_constant_fold(&mut self, expr: &CoreExpr) -> CoreExpr {
        if let CoreExprNode::App(func, arg) = &expr.node {
            if let CoreExprNode::App(inner_func, inner_arg) = &func.node {
                if let CoreExprNode::Var(op) = &inner_func.node {
                    if let CoreExprNode::Lit(Literal::I64(a)) = &inner_arg.node {
                        if let CoreExprNode::Lit(Literal::I64(b)) = &arg.node {
                            let result = match op.as_str() {
                                "+" => Some(*a + *b),
                                "-" => Some(*a - *b),
                                "*" => Some(*a * *b),
                                "/" => if *b != 0 { Some(*a / *b) } else { None },
                                _ => None,
                            };
                            if let Some(n) = result {
                                self.stats.folded += 1;
                                return CoreExpr::new(CoreExprNode::Lit(Literal::I64(n)), expr.span);
                            }
                        }
                    }
                }
            }
        }
        expr.clone()
    }

    /// Check if a variable is used in an expression
    fn uses_var(&self, name: &Symbol, expr: &CoreExpr) -> bool {
        match &expr.node {
            CoreExprNode::Var(v) => v == name,
            CoreExprNode::Lam(lambda) => {
                if lambda.params.iter().any(|p| &p.name == name) {
                    false
                } else {
                    self.uses_var(name, &lambda.body)
                }
            }
            CoreExprNode::App(f, a) => self.uses_var(name, f) || self.uses_var(name, a),
            CoreExprNode::Let(n, _, v, b) => {
                if n == name {
                    self.uses_var(name, v) || self.uses_var(name, b)
                } else {
                    self.uses_var(name, v) || self.uses_var(name, b)
                }
            }
            CoreExprNode::If(c, t, e) => {
                self.uses_var(name, c) || self.uses_var(name, t) || self.uses_var(name, e)
            }
            CoreExprNode::Match(s, arms) => {
                self.uses_var(name, s) || arms.iter().any(|arm| {
                    if self.pattern_binds_var(&arm.pattern, name) {
                        false
                    } else {
                        self.uses_var(name, &arm.body)
                    }
                })
            }
            CoreExprNode::Data(_, args) => args.iter().any(|a| self.uses_var(name, a)),
            CoreExprNode::Handle(b, _) => self.uses_var(name, b),
            CoreExprNode::Perform(_, args) => args.iter().any(|a| self.uses_var(name, a)),
            CoreExprNode::Do(exprs) => exprs.iter().any(|e| self.uses_var(name, e)),
            _ => false,
        }
    }

    /// Collect names used across all definitions
    fn collect_used_names(&self, defs: &[CoreDef]) -> std::collections::HashSet<Symbol> {
        let mut used = std::collections::HashSet::new();
        for def in defs {
            self.collect_names_from_expr(&def.body, &mut used);
        }
        // "main" is always considered used
        used.insert(Symbol::new("main"));
        used
    }

    fn collect_names_from_expr(&self, expr: &CoreExpr, names: &mut std::collections::HashSet<Symbol>) {
        match &expr.node {
            CoreExprNode::Var(name) => { names.insert(name.clone()); }
            CoreExprNode::Lam(l) => self.collect_names_from_expr(&l.body, names),
            CoreExprNode::App(f, a) => {
                self.collect_names_from_expr(f, names);
                self.collect_names_from_expr(a, names);
            }
            CoreExprNode::Let(_, _, v, b) => {
                self.collect_names_from_expr(v, names);
                self.collect_names_from_expr(b, names);
            }
            CoreExprNode::If(c, t, e) => {
                self.collect_names_from_expr(c, names);
                self.collect_names_from_expr(t, names);
                self.collect_names_from_expr(e, names);
            }
            CoreExprNode::Match(s, arms) => {
                self.collect_names_from_expr(s, names);
                for arm in arms {
                    if let Some(g) = &arm.guard {
                        self.collect_names_from_expr(g, names);
                    }
                    self.collect_names_from_expr(&arm.body, names);
                }
            }
            CoreExprNode::Data(_, args) => {
                for a in args { self.collect_names_from_expr(a, names); }
            }
            CoreExprNode::Handle(b, _) => self.collect_names_from_expr(b, names),
            CoreExprNode::Perform(_, args) => {
                for a in args { self.collect_names_from_expr(a, names); }
            }
            _ => {}
        }
    }
}

/// Check if an expression has side effects
fn has_side_effects(expr: &CoreExpr) -> bool {
    match &expr.node {
        CoreExprNode::Perform(_, _) => true,
        CoreExprNode::Handle(_, _) => true,
        CoreExprNode::App(f, _) => has_side_effects(f),
        CoreExprNode::Let(_, _, v, _) => has_side_effects(v),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tisp_core::types::{Grade, Mode, Determinism, EffectRow};

    fn var(name: &str) -> CoreExpr { CoreExpr::new(CoreExprNode::Var(Symbol::new(name)), Span::dummy()) }
    fn int(n: i64) -> CoreExpr { CoreExpr::new(CoreExprNode::Lit(Literal::I64(n)), Span::dummy()) }
    fn app(f: CoreExpr, a: CoreExpr) -> CoreExpr {
        CoreExpr::new(CoreExprNode::App(Box::new(f), Box::new(a)), Span::dummy())
    }

    fn make_def(name: &str, params: Vec<&str>, body: CoreExpr) -> CoreDef {
        CoreDef {
            name: Symbol::new(name),
            ty: None,
            effects: EffectRow::Pure,
            grade: Grade::Omega,
            mode: Mode::In,
            region: None,
            visibility: Visibility::Public,
            mode_sigs: vec![],
            determinism: Determinism::Det,
            body: CoreExpr::new(CoreExprNode::Lam(Lambda {
                params: params.into_iter().map(|p| Param { name: Symbol::new(p), ty: None, grade: Grade::Omega, mode: Mode::In }).collect(),
                body: Box::new(body),
                ret_type: None,
            }), Span::dummy()),
            requires: None,
            ensures: None,
            span: Span::dummy(),
        }
    }

    fn make_program(defs: Vec<CoreDef>, pragmas: Vec<(Symbol, Vec<Symbol>)>) -> CoreProgram {
        CoreProgram {
            data_decls: vec![], effect_decls: vec![], type_families: vec![], resource_algebras: vec![],
            defs, pragmas,
        }
    }

    /// §30 inline!:强制内联(忽略阈值);double 体大小 6 > 默认阈值 5
    #[test]
    fn test_optimizer_pragma_inline_force() {
        let double = make_def("double", vec!["x"], app(app(var("+"), var("x")), var("x")));
        let main_def = make_def("main", vec![], app(var("double"), int(21)));
        let prog = make_program(vec![double, main_def], vec![(Symbol::new("inline!"), vec![Symbol::new("double")])]);
        let mut opt = Optimizer::new();
        opt.configure(&prog.pragmas);
        let _ = opt.optimize(&prog);
        assert!(opt.stats.inlined >= 1, "inline! 应强制内联 double");
    }

    /// §30 opt-level 0:关闭内联
    #[test]
    fn test_optimizer_opt_level_zero() {
        let double = make_def("double", vec!["x"], app(app(var("+"), var("x")), var("x")));
        let main_def = make_def("main", vec![], app(var("double"), int(21)));
        let prog = make_program(vec![double, main_def], vec![(Symbol::new("opt-level"), vec![Symbol::new("0")])]);
        let mut opt = Optimizer::new();
        opt.configure(&prog.pragmas);
        let _ = opt.optimize(&prog);
        assert_eq!(opt.stats.inlined, 0, "opt-level 0 应关闭内联");
    }
}
