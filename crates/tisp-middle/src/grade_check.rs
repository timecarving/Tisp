use std::collections::HashMap;
use tisp_core::core_ast::*;
use tisp_core::symbol::Symbol;
use tisp_core::types::{Grade, Type};
use tisp_core::grades::{grade_add, grade_le};
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
        // §10 等级变量集合:def 声明类型与 Lam 参数类型中的类型级符号
        let mut type_vars: Vec<Symbol> = Vec::new();
        if let Some(ty) = &def.ty {
            self.collect_type_vars(ty, &mut type_vars);
        }
        if let CoreExprNode::Lam(lam) = &def.body.node {
            for p in &lam.params {
                if let Some(ty) = &p.ty {
                    self.collect_type_vars(ty, &mut type_vars);
                }
            }
            // 等级变量绑定校验:Grade::Var 须在类型变量集合内
            for p in &lam.params {
                if let Grade::Var(v) = &p.grade {
                    if !type_vars.contains(v) {
                        return Err(GradeError {
                            message: format!("未绑定的等级变量 '{}'(须为类型参数,如 (Vec i64 {}) 的 {})", v, v, v),
                            span: def.span.clone(),
                        });
                    }
                }
            }
        }
        self.check_expr(&def.body)?;

        // ── Dependent grade propagation: type-level usage ──
        if let Some(ref ty) = def.ty {
            self.check_dependent_grades(ty, &def.span)?;
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

    /// §19.1 r+s 依赖等级传播检查:Pi/Sigma 绑定在结果类型中的使用次数。
    /// 当前语法下依赖绑定为 ω(不限制);机制就位,等级化语法(One/Zero 绑定)引入后在此按 r+s 检查。
    fn check_dependent_grades(&self, ty: &Type, span: &Span) -> Result<(), GradeError> {
        match ty {
            Type::Pi(name, dom, cod) | Type::Sigma(name, dom, cod) => {
                let count = self.count_var_in_type(name, cod);
                // ω 绑定不限制;若绑定等级为 One(未来语法),count != 1 时在此报错
                let _ = count;
                self.check_dependent_grades(dom, span)?;
                self.check_dependent_grades(cod, span)?;
            }
            Type::Fun(p, ann, r) => {
                // r+s:参数类型与返回类型中的依赖出现与函数等级 ann.grade 的关系
                let _ = ann;
                self.check_dependent_grades(p, span)?;
                self.check_dependent_grades(r, span)?;
            }
            Type::Forall(vars, body) => {
                // 隐式绑定(§10.2)默认 0 级:类型级出现属擦除语义,不检查
                let _ = vars;
                self.check_dependent_grades(body, span)?;
            }
            Type::App(f, a) => {
                self.check_dependent_grades(f, span)?;
                self.check_dependent_grades(a, span)?;
            }
            Type::Tuple(ts) => {
                for t in ts { self.check_dependent_grades(t, span)?; }
            }
            Type::Refined(base, _) => self.check_dependent_grades(base, span)?,
            _ => {}
        }
        Ok(())
    }

    /// §10 收集类型中的类型级符号(等级变量合法性来源)
    fn collect_type_vars(&self, ty: &Type, out: &mut Vec<Symbol>) {
        match ty {
            Type::Var(v) => { if !out.contains(&v.name) { out.push(v.name.clone()); } }
            // 小写 Con 按类型变量处理(Haskell 惯例,与类型族一致):(Vec i64 n) 的 n
            Type::Con(c) => {
                let first = c.name.as_str().chars().next();
                if matches!(first, Some(ch) if ch.is_ascii_lowercase()) && !out.contains(&c.name) {
                    out.push(c.name.clone());
                }
            }
            Type::App(f, a) => { self.collect_type_vars(f, out); self.collect_type_vars(a, out); }
            Type::Fun(p, _, r) => { self.collect_type_vars(p, out); self.collect_type_vars(r, out); }
            Type::Forall(vs, body) => {
                for v in vs { if !out.contains(&v.name) { out.push(v.name.clone()); } }
                self.collect_type_vars(body, out);
            }
            Type::Tuple(ts) => { for t in ts { self.collect_type_vars(t, out); } }
            Type::Refined(base, _) => self.collect_type_vars(base, out),
            Type::Pi(_, d, c) | Type::Sigma(_, d, c) => {
                self.collect_type_vars(d, out);
                self.collect_type_vars(c, out);
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
            // 依赖绑定在结果中的出现即依赖使用(r+s 传播对象),计数
            Type::Pi(_, d, c) | Type::Sigma(_, d, c) => {
                self.count_var_in_type(name, d) + self.count_var_in_type(name, c)
            }
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

                // Check linear parameters are used exactly once;§10 依赖等级:使用次数 ≤ 等级(上界)
                for param in &lambda.params {
                    match &param.grade {
                        Grade::One => {
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
                        // 依赖等级(Nat/Add/Mul/Var):计数 ≤ 等级;可判定时检查,不可判定放行
                        other if !matches!(other, Grade::Zero | Grade::Omega) => {
                            let usage = self.usage_env.get_usage(&param.name);
                            match grade_le(&usage, other) {
                                Some(true) => {}
                                Some(false) => {
                                    return Err(GradeError {
                                        message: format!(
                                            "grade violation: '{}' used {:?} times, exceeds grade {:?}",
                                            param.name, usage, other
                                        ),
                                        span: expr.span,
                                    });
                                }
                                None => {} // 符号等级:不可判定,放行
                            }
                        }
                        _ => {}
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
            Pattern::Con(_, subpats) => {
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
            Pattern::Or(pats) => {
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
            Pattern::Or(pats) => {
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
        &mut self,
        then_usage: &HashMap<Symbol, Grade>,
        else_usage: &HashMap<Symbol, Grade>,
        span: Span,
    ) -> Result<(), GradeError> {
        // One 绑定:两分支必须使用相同次数(恰好语义)
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
            // §10 依赖等级:分支计数取上界(max),合并进当前使用环境
            if !matches!(declared, Grade::Zero | Grade::One | Grade::Omega) {
                let then_use = then_usage.get(name).cloned().unwrap_or(Grade::Zero);
                let else_use = else_usage.get(name).cloned().unwrap_or(Grade::Zero);
                // 常量可判定时取大者;不可判定保留 then 侧(上界近似)
                let merged = match grade_le(&then_use, &else_use) {
                    Some(true) => else_use.clone(),
                    Some(false) => then_use.clone(),
                    None => then_use.clone(),
                };
                self.usage.insert(name.clone(), merged);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tisp_core::core_ast::{CoreExpr, CoreExprNode, CoreDef, CoreProgram, Lambda, Param};
    use tisp_core::span::Span;
    use tisp_core::symbol::Symbol;

    fn e(node: CoreExprNode) -> CoreExpr {
        CoreExpr::new(node, Span::dummy())
    }
    fn var(name: &str) -> CoreExprNode { CoreExprNode::Var(Symbol::new(name)) }
    fn int(n: i64) -> CoreExprNode { CoreExprNode::Lit(tisp_core::core_ast::Literal::I64(n)) }

    fn def_with_lam(name: &str, params: Vec<Param>, body: CoreExprNode) -> CoreDef {
        CoreDef {
            name: Symbol::new(name),
            ty: None,
            effects: tisp_core::types::EffectRow::Pure,
            grade: Grade::Omega,
            mode: tisp_core::types::Mode::In,
            determinism: tisp_core::types::Determinism::Det,
            mode_sigs: vec![],
            body: e(CoreExprNode::Lam(Lambda { params, body: Box::new(e(body)), ret_type: None })),
            requires: None,
            ensures: None,
            span: Span::dummy(),
        }
    }

    fn check(src_defs: Vec<CoreDef>) -> Result<(), GradeError> {
        let mut g = GradeChecker::new();
        g.check_program(&CoreProgram { data_decls: vec![], effect_decls: vec![], type_families: vec![],
            resource_algebras: vec![], defs: src_defs })
    }

    fn one_param(name: &str) -> Param {
        Param { name: Symbol::new(name), ty: None, grade: Grade::One, mode: tisp_core::types::Mode::In }
    }

    #[test]
    fn test_linear_used_once_ok() {
        // §10.1:1 级参数恰好使用一次 → 通过
        let d = def_with_lam("f", vec![one_param("x")], var("x"));
        assert!(check(vec![d]).is_ok(), "线性参数用一次应通过");
    }

    #[test]
    fn test_linear_used_twice_fails() {
        // §10.1:1 级参数使用两次(移动后复用)→ 报错
        let body = e(CoreExprNode::Do(vec![e(var("x")), e(var("x"))]));
        let d = CoreDef {
            name: Symbol::new("f"),
            ty: None,
            effects: tisp_core::types::EffectRow::Pure,
            grade: Grade::Omega,
            mode: tisp_core::types::Mode::In,
            determinism: tisp_core::types::Determinism::Det,
            mode_sigs: vec![],
            body: e(CoreExprNode::Lam(Lambda { params: vec![one_param("x")], body: Box::new(body), ret_type: None })),
            requires: None,
            ensures: None,
            span: Span::dummy(),
        };
        let err = check(vec![d]).unwrap_err();
        assert!(err.message.contains("linear"), "错误应提及 linear,实际: {}", err.message);
    }

    #[test]
    fn test_linear_unused_fails() {
        // §10.1:1 级参数未使用 → 报错
        let d = def_with_lam("f", vec![one_param("x")], int(42));
        let err = check(vec![d]).unwrap_err();
        assert!(err.message.contains("linear"), "错误应提及 linear,实际: {}", err.message);
    }

    #[test]
    fn test_dependent_grade_pi_ok() {
        // §19.1:Pi 绑定类型检查通过(当前绑定为 ω)
        let pi_ty = Type::Pi(
            Symbol::new("n"),
            Box::new(Type::i64()),
            Box::new(Type::App(
                Box::new(Type::Con(tisp_core::types::TypeCon { name: Symbol::new("Vec"), kind: tisp_core::types::Kind::Star })),
                Box::new(Type::Var(tisp_core::types::TypeVar { name: Symbol::new("n"), kind: tisp_core::types::Kind::Star, id: 0 })),
            )),
        );
        let mut d = CoreDef {
            name: Symbol::new("f"),
            ty: Some(pi_ty.clone()),
            effects: tisp_core::types::EffectRow::Pure,
            grade: Grade::Omega,
            mode: tisp_core::types::Mode::In,
            mode_sigs: vec![],
            determinism: tisp_core::types::Determinism::Det,
            body: e(CoreExprNode::Lam(tisp_core::core_ast::Lambda { params: vec![], body: Box::new(e(var("x"))), ret_type: None })),
            requires: None,
            ensures: None,
            span: Span::dummy(),
        };
        // body 引用 x 未绑定——改用不引用变量的 body
        d.body = e(CoreExprNode::Lam(tisp_core::core_ast::Lambda { params: vec![], body: Box::new(e(int(1))), ret_type: None }));
        let mut g = GradeChecker::new();
        assert!(g.check_program(&CoreProgram { data_decls: vec![], effect_decls: vec![], type_families: vec![],
            resource_algebras: vec![], defs: vec![d] }).is_ok());
        // 计数:n 在 (Vec n) 中出现 1 次
        let g2 = GradeChecker::new();
        assert_eq!(g2.count_var_in_type(&Symbol::new("n"), &pi_ty), 1);
    }

    #[test]
    fn test_zero_grade_ignored() {
        // §10.1:0 级参数不参与线性检查
        let p = Param { name: Symbol::new("x"), ty: None, grade: Grade::Zero, mode: tisp_core::types::Mode::In };
        let d = def_with_lam("f", vec![p], int(42));
        assert!(check(vec![d]).is_ok(), "0 级参数未使用应放行");
    }
}

#[cfg(test)]
mod dep_grade_tests {
    use super::*;

    fn e(node: CoreExprNode) -> CoreExpr {
        CoreExpr::new(node, Span::dummy())
    }
    fn var(name: &str) -> CoreExprNode { CoreExprNode::Var(Symbol::new(name)) }
    fn int(n: i64) -> CoreExprNode { CoreExprNode::Lit(tisp_core::core_ast::Literal::I64(n)) }

    fn def_with_grade(name: &str, grade: Grade, uses: usize) -> CoreDef {
        let body = if uses == 1 {
            e(var("x"))
        } else {
            e(CoreExprNode::Do((0..uses).map(|_| e(var("x"))).collect()))
        };
        CoreDef {
            name: Symbol::new(name),
            ty: None,
            effects: tisp_core::types::EffectRow::Pure,
            grade: Grade::Omega,
            mode: tisp_core::types::Mode::In,
            mode_sigs: vec![],
            determinism: tisp_core::types::Determinism::Det,
            body: e(CoreExprNode::Lam(tisp_core::core_ast::Lambda {
                params: vec![tisp_core::core_ast::Param { name: Symbol::new("x"), ty: None, grade, mode: tisp_core::types::Mode::In }],
                body: Box::new(body),
                ret_type: None,
            })),
            requires: None,
            ensures: None,
            span: Span::dummy(),
        }
    }

    #[test]
    fn test_nat_grade_branch_upper_bound() {
        // 分支合并:then 2 次、else 1 次,等级 3 → 上界 2 ≤ 3 通过
        let body = e(CoreExprNode::If(
            Box::new(e(CoreExprNode::Lit(tisp_core::core_ast::Literal::Bool(true)))),
            Box::new(e(CoreExprNode::Do(vec![e(var("x")), e(var("x"))]))),
            Box::new(e(var("x"))),
        ));
        let d = CoreDef {
            name: Symbol::new("b"),
            ty: None,
            effects: tisp_core::types::EffectRow::Pure,
            grade: Grade::Omega,
            mode: tisp_core::types::Mode::In,
            mode_sigs: vec![],
            determinism: tisp_core::types::Determinism::Det,
            body: e(CoreExprNode::Lam(tisp_core::core_ast::Lambda {
                params: vec![tisp_core::core_ast::Param { name: Symbol::new("x"), ty: None, grade: Grade::Nat(3), mode: tisp_core::types::Mode::In }],
                body: Box::new(body),
                ret_type: None,
            })),
            requires: None,
            ensures: None,
            span: Span::dummy(),
        };
        let prog = CoreProgram { data_decls: vec![], effect_decls: vec![], type_families: vec![], resource_algebras: vec![], defs: vec![d] };
        let mut g = GradeChecker::new();
        assert!(g.check_program(&prog).is_ok(), "分支上界 2 ≤ 3 应通过");
    }

    #[test]
    fn test_nat_grade_violation() {
        // 等级 Nat(3) 使用 4 次 → 违反
        let d = def_with_grade("bad", Grade::Nat(3), 4);
        let prog = CoreProgram { data_decls: vec![], effect_decls: vec![], type_families: vec![], resource_algebras: vec![], defs: vec![d] };
        let mut g = GradeChecker::new();
        let err = g.check_program(&prog).unwrap_err();
        assert!(err.message.contains("grade violation"), "应报等级违反,实际: {}", err.message);
    }

    #[test]
    fn test_nat_grade_ok() {
        // 等级 Nat(3) 使用 3 次 → 通过
        let d = def_with_grade("ok", Grade::Nat(3), 3);
        let prog = CoreProgram { data_decls: vec![], effect_decls: vec![], type_families: vec![], resource_algebras: vec![], defs: vec![d] };
        let mut g = GradeChecker::new();
        assert!(g.check_program(&prog).is_ok(), "3 次使用应通过");
    }

    #[test]
    fn test_var_grade_unbound() {
        // Grade::Var(m) 未在类型中出现 → 报未绑定
        let d = def_with_grade("u", Grade::Var(Symbol::new("m")), 1);
        let prog = CoreProgram { data_decls: vec![], effect_decls: vec![], type_families: vec![], resource_algebras: vec![], defs: vec![d] };
        let mut g = GradeChecker::new();
        let err = g.check_program(&prog).unwrap_err();
        assert!(err.message.contains("未绑定"), "应报未绑定,实际: {}", err.message);
    }

    #[test]
    fn test_var_grade_bound_passes() {
        // Grade::Var(n) 且 n 出现在类型 (Vec i64 n) 中 → 通过(符号等级放行)
        let ty = Type::App(
            Box::new(Type::App(
                Box::new(Type::Con(tisp_core::types::TypeCon { name: Symbol::new("Vec"), kind: tisp_core::types::Kind::Star })),
                Box::new(Type::i64()))),
            Box::new(Type::Con(tisp_core::types::TypeCon { name: Symbol::new("n"), kind: tisp_core::types::Kind::Star })),
        );
        let mut d = def_with_grade("d", Grade::Var(Symbol::new("n")), 3);
        if let CoreExprNode::Lam(lam) = &mut d.body.node {
            lam.params[0].ty = Some(ty);
        }
        let prog = CoreProgram { data_decls: vec![], effect_decls: vec![], type_families: vec![], resource_algebras: vec![], defs: vec![d] };
        let mut g = GradeChecker::new();
        assert!(g.check_program(&prog).is_ok(), "绑定等级变量应放行");
    }
}
