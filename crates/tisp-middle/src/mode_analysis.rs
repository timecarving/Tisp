use tisp_core::core_ast::*;
use tisp_core::symbol::Symbol;
use tisp_core::types::Mode;

use tisp_core::span::Span;

pub struct ModeAnalyzer {
    pub mode_env: std::collections::HashMap<Symbol, Mode>,
    /// 多模式谓词签名表(§13):谓词名 → 模式签名列表(每个签名是参数 Mode 列表)
    mode_sigs: std::collections::HashMap<Symbol, Vec<Vec<Mode>>>,
    /// §多模式自动推断:未声明 :mode 的谓词按调用形态收集的模式
    pub inferred_sigs: std::collections::HashMap<Symbol, Vec<Vec<Mode>>>,
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
        Self {
            mode_env: std::collections::HashMap::new(),
            mode_sigs: std::collections::HashMap::new(),
            inferred_sigs: std::collections::HashMap::new(),
        }
    }

    pub fn analyze_program(&mut self, program: &CoreProgram) -> Result<Vec<(Symbol, Mode)>, ModeError> {
        // 先收集多模式签名(调用点可能引用后续 def);§13.3 同名多模式重载:合并而非覆盖
        for def in &program.defs {
            if !def.mode_sigs.is_empty() {
                let entry = self.mode_sigs.entry(def.name.clone()).or_default();
                for sig in &def.mode_sigs {
                    if !entry.contains(sig) {
                        entry.push(sig.clone());
                    }
                }
            }
        }
        let mut results = Vec::new();
        for def in &program.defs {
            // 调用点模式匹配检查(§13)
            self.check_call_sites(def)?;
            let mode = self.analyze_def(def)?;
            self.mode_env.insert(def.name.clone(), mode.clone());
            results.push((def.name.clone(), mode));
        }
        Ok(results)
    }

    /// 调用点检查:对声明了 :mode 签名的 defpred 调用,实参模式须匹配某个签名
    fn check_call_sites(&mut self, def: &CoreDef) -> Result<(), ModeError> {
        let mut calls = Vec::new();
        walk_calls(&def.body, &mut calls);
        for (callee, args) in calls {
            // 实参模式:含 Fresh/逻辑变量 → Out(free),否则 In(ground)
            let arg_modes: Vec<Mode> = args.iter()
                .map(|a| if expr_has_fresh(a) { Mode::Out } else { Mode::In })
                .collect();
            let Some(sigs) = self.mode_sigs.get(&callee) else {
                // §多模式自动推断:未声明 :mode 的谓词按调用形态收集模式
                let inferred = self.inferred_sigs.entry(callee.clone()).or_default();
                if !inferred.contains(&arg_modes) {
                    inferred.push(arg_modes);
                }
                continue;
            };
            let matched = sigs.iter().any(|sig| {
                sig.len() == arg_modes.len() && sig.iter().zip(&arg_modes).all(|(s, a)| s == a)
            });
            if !matched && sigs.iter().any(|sig| sig.len() == arg_modes.len()) {
                let span = args.last().map(|a| a.span.clone()).unwrap_or_else(|| def.span.clone());
                return Err(ModeError {
                    message: format!(
                        "谓词 '{}' 调用无匹配模式:实参 {:?},可用模式 {:?}",
                        callee, arg_modes, sigs
                    ),
                    span,
                });
            }
        }
        Ok(())
    }

    fn analyze_def(&mut self, def: &CoreDef) -> Result<Mode, ModeError> {
        // §13 自动模式推断:未声明 :mode 的谓词从函数体推断参数模式(接线 infer_modes)
        if def.mode_sigs.is_empty() {
            if let CoreExprNode::Lam(lam) = &def.body.node {
                if !lam.params.is_empty() {
                    let inferred = self.infer_modes(&lam.params, &lam.body)?;
                    let sigs = self.inferred_sigs.entry(def.name.clone()).or_default();
                    if !sigs.contains(&inferred) {
                        sigs.push(inferred);
                    }
                }
            }
        }
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
            Pattern::Or(pats) => pats.iter().any(|p| self.pattern_binds(var, p)),
        }
    }
}

/// 收集表达式中的直接调用:(callee, args)(柯里化链展开,递归嵌套)
fn walk_calls(expr: &CoreExpr, out: &mut Vec<(Symbol, Vec<CoreExpr>)>) {
    match &expr.node {
        CoreExprNode::App(_, _) => {
            // 只收集完整柯里化链(内部 App 不重复收集);递归实参找嵌套调用
            if let Some((name, args)) = collect_chain(expr) {
                for a in &args {
                    walk_calls(a, out);
                }
                out.push((name, args));
            }
        }
        CoreExprNode::If(c, t, e) => { walk_calls(c, out); walk_calls(t, out); walk_calls(e, out); }
        CoreExprNode::Let(_, _, v, body) => { walk_calls(v, out); walk_calls(body, out); }
        CoreExprNode::Do(items) => { for i in items { walk_calls(i, out); } }
        CoreExprNode::Lam(lam) => walk_calls(&lam.body, out),
        CoreExprNode::Match(s, arms) => {
            walk_calls(s, out);
            for arm in arms { walk_calls(&arm.body, out); }
        }
        CoreExprNode::Data(_, args) => { for a in args { walk_calls(a, out); } }
        CoreExprNode::Perform(_, args) => { for a in args { walk_calls(a, out); } }
        CoreExprNode::Search(e) => walk_calls(e, out),
        CoreExprNode::Unify(a, b) => { walk_calls(a, out); walk_calls(b, out); }
        _ => {}
    }
}

/// 展开柯里化调用链:App(App(App(Var(f), a1), a2), a3) → (f, [a1, a2, a3])
fn collect_chain(expr: &CoreExpr) -> Option<(Symbol, Vec<CoreExpr>)> {
    match &expr.node {
        CoreExprNode::Var(name) => Some((name.clone(), Vec::new())),
        CoreExprNode::App(f, arg) => {
            collect_chain(f).map(|(name, mut args)| {
                args.push((**arg).clone());
                (name, args)
            })
        }
        _ => None,
    }
}

/// 表达式是否含 Fresh(逻辑变量创建)→ 实参模式判定为 Out
fn expr_has_fresh(expr: &CoreExpr) -> bool {
    match &expr.node {
        CoreExprNode::Fresh(_) => true,
        CoreExprNode::App(f, a) => expr_has_fresh(f) || expr_has_fresh(a),
        CoreExprNode::Do(items) => items.iter().any(expr_has_fresh),
        CoreExprNode::Search(e) => expr_has_fresh(e),
        CoreExprNode::Data(_, args) => args.iter().any(expr_has_fresh),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(node: CoreExprNode) -> CoreExpr {
        CoreExpr::new(node, Span::dummy())
    }
    fn var(name: &str) -> CoreExprNode { CoreExprNode::Var(Symbol::new(name)) }
    fn int(n: i64) -> CoreExprNode { CoreExprNode::Lit(tisp_core::core_ast::Literal::I64(n)) }

    fn pred_def(name: &str, mode_sigs: Vec<Vec<Mode>>) -> CoreDef {
        CoreDef {
            name: Symbol::new(name),
            ty: None,
            effects: tisp_core::types::EffectRow::Pure,
            grade: tisp_core::types::Grade::Omega,
            mode: Mode::Free,
            mode_sigs,
            determinism: tisp_core::types::Determinism::NonDet,
            region: None,
            visibility: Visibility::Public,
            body: e(CoreExprNode::Lam(tisp_core::core_ast::Lambda {
                params: vec![
                    tisp_core::core_ast::Param { name: Symbol::new("x"), ty: None, grade: tisp_core::types::Grade::Omega, mode: Mode::In },
                    tisp_core::core_ast::Param { name: Symbol::new("y"), ty: None, grade: tisp_core::types::Grade::Omega, mode: Mode::In },
                ],
                body: Box::new(e(var("y"))),
                ret_type: None,
            })),
            requires: None,
            ensures: None,
            span: Span::dummy(),
        }
    }

    fn caller_def(name: &str, body: CoreExprNode) -> CoreDef {
        CoreDef {
            name: Symbol::new(name),
            ty: None,
            effects: tisp_core::types::EffectRow::Pure,
            grade: tisp_core::types::Grade::Omega,
            mode: Mode::In,
            mode_sigs: vec![],
            determinism: tisp_core::types::Determinism::Det,
            region: None,
            visibility: Visibility::Public,
            body: e(CoreExprNode::Lam(tisp_core::core_ast::Lambda {
                params: vec![],
                body: Box::new(e(body)),
                ret_type: None,
            })),
            requires: None,
            ensures: None,
            span: Span::dummy(),
        }
    }

    fn call(callee: &str, args: Vec<CoreExprNode>) -> CoreExprNode {
        // (f a b) = App(App(Var(f), a), b)
        let mut node = CoreExprNode::Var(Symbol::new(callee));
        for a in args {
            node = CoreExprNode::App(Box::new(e(node)), Box::new(e(a)));
        }
        node
    }

    #[test]
    fn test_mode_sig_match_ok() {
        // (p 1 (fresh x)):实参 [In, Out] 匹配签名 [In, Out]
        let p = pred_def("p", vec![vec![Mode::In, Mode::Out], vec![Mode::Out, Mode::In]]);
        let main = caller_def("main", call("p", vec![int(1), CoreExprNode::Fresh(Symbol::new("x"))]));
        let prog = CoreProgram { data_decls: vec![], effect_decls: vec![], type_families: vec![],
            resource_algebras: vec![], defs: vec![p, main], pragmas: vec![] };
        let mut m = ModeAnalyzer::new();
        assert!(m.analyze_program(&prog).is_ok(), "实参模式应匹配签名");
    }

    #[test]
    fn test_mode_sig_mismatch_fails() {
        // 谓词只有 [In, Out],调用 (p 1 2) 实参 [In, In] → 无匹配
        let p = pred_def("p", vec![vec![Mode::In, Mode::Out]]);
        let main = caller_def("main", call("p", vec![int(1), int(2)]));
        let prog = CoreProgram { data_decls: vec![], effect_decls: vec![], type_families: vec![],
            resource_algebras: vec![], defs: vec![p, main], pragmas: vec![] };
        let mut m = ModeAnalyzer::new();
        let err = m.analyze_program(&prog).unwrap_err();
        assert!(err.message.contains("无匹配模式"), "错误应提及模式,实际: {}", err.message);
    }

    #[test]
    fn test_no_mode_sigs_skipped() {
        // 无 :mode 声明 → 不检查
        let q = pred_def("q", vec![]);
        let main = caller_def("main", call("q", vec![int(1), int(2)]));
        let prog = CoreProgram { data_decls: vec![], effect_decls: vec![], type_families: vec![],
            resource_algebras: vec![], defs: vec![q, main], pragmas: vec![] };
        let mut m = ModeAnalyzer::new();
        assert!(m.analyze_program(&prog).is_ok());
    }

    #[test]
    fn test_same_name_multi_mode_overload() {
        // §13.3:同名谓词两个 defpred,不同 :mode → 签名合并(非覆盖)
        let p1 = pred_def("member", vec![vec![Mode::In, Mode::Out]]);
        let p2 = pred_def("member", vec![vec![Mode::Out, Mode::In]]);
        // 调用 (member 1 (fresh x)) = [In, Out] 匹配 p1 签名;旧代码被 p2 覆盖会误报
        let main = caller_def("main", call("member", vec![int(1), CoreExprNode::Fresh(Symbol::new("x"))]));
        let prog = CoreProgram { data_decls: vec![], effect_decls: vec![], type_families: vec![],
            resource_algebras: vec![], defs: vec![p1, p2, main], pragmas: vec![] };
        let mut m = ModeAnalyzer::new();
        assert!(m.analyze_program(&prog).is_ok(), "同名多模式应合并,调用应匹配任一签名");
    }
}

#[cfg(test)]
mod infer_tests {
    use super::*;

    fn e(node: CoreExprNode) -> CoreExpr {
        CoreExpr::new(node, Span::dummy())
    }
    fn var(name: &str) -> CoreExprNode { CoreExprNode::Var(Symbol::new(name)) }
    fn int(n: i64) -> CoreExprNode { CoreExprNode::Lit(tisp_core::core_ast::Literal::I64(n)) }

    fn pred_def(name: &str) -> CoreDef {
        CoreDef {
            name: Symbol::new(name),
            ty: None,
            effects: tisp_core::types::EffectRow::Pure,
            grade: tisp_core::types::Grade::Omega,
            mode: Mode::Free,
            mode_sigs: vec![],
            determinism: tisp_core::types::Determinism::Det,
            region: None,
            visibility: Visibility::Public,
            body: e(CoreExprNode::Lam(tisp_core::core_ast::Lambda {
                params: vec![],
                body: Box::new(e(var("y"))),
                ret_type: None,
            })),
            requires: None,
            ensures: None,
            span: Span::dummy(),
        }
    }

    fn caller(body: CoreExprNode) -> CoreDef {
        CoreDef {
            name: Symbol::new("main"),
            ty: None,
            effects: tisp_core::types::EffectRow::Pure,
            grade: tisp_core::types::Grade::Omega,
            mode: Mode::In,
            mode_sigs: vec![],
            determinism: tisp_core::types::Determinism::Det,
            region: None,
            visibility: Visibility::Public,
            body: e(CoreExprNode::Lam(tisp_core::core_ast::Lambda {
                params: vec![],
                body: Box::new(e(body)),
                ret_type: None,
            })),
            requires: None,
            ensures: None,
            span: Span::dummy(),
        }
    }

    fn call(callee: &str, args: Vec<CoreExprNode>) -> CoreExprNode {
        let mut node = CoreExprNode::Var(Symbol::new(callee));
        for a in args {
            node = CoreExprNode::App(Box::new(e(node)), Box::new(e(a)));
        }
        node
    }

    #[test]
    fn test_mode_auto_inference() {
        // §13:未声明 :mode 的谓词按调用形态自动推断
        let p = pred_def("p");
        // 调用 (p 1 2):实参 [In, In]
        let main = caller(call("p", vec![int(1), int(2)]));
        let prog = CoreProgram { data_decls: vec![], effect_decls: vec![], type_families: vec![], resource_algebras: vec![], defs: vec![p, main], pragmas: vec![] };
        let mut m = ModeAnalyzer::new();
        assert!(m.analyze_program(&prog).is_ok());
        let inferred = m.inferred_sigs.get(&Symbol::new("p")).cloned().unwrap_or_default();
        assert_eq!(inferred, vec![vec![Mode::In, Mode::In]], "应推断 [In, In],实际 {:?}", inferred);

        // 两种形态:p 被 (p 1 2) 与含 free 实参调用 → 推断两个签名
        let p2 = pred_def("q");
        let main2 = caller(CoreExprNode::Do(vec![
            e(call("q", vec![int(1), int(2)])),
            e(call("q", vec![CoreExprNode::Fresh(Symbol::new("z")), int(1)])),
        ]));
        let prog2 = CoreProgram { data_decls: vec![], effect_decls: vec![], type_families: vec![], resource_algebras: vec![], defs: vec![p2, main2], pragmas: vec![] };
        let mut m2 = ModeAnalyzer::new();
        assert!(m2.analyze_program(&prog2).is_ok());
        let inferred2 = m2.inferred_sigs.get(&Symbol::new("q")).cloned().unwrap_or_default();
        assert_eq!(inferred2.len(), 2, "应推断两个模式,实际 {:?}", inferred2);
        assert!(inferred2.contains(&vec![Mode::In, Mode::In]));
        assert!(inferred2.contains(&vec![Mode::Out, Mode::In]));
    }

    #[test]
    fn test_body_based_mode_inference() {
        // §13:未声明 :mode 的谓词从函数体推断参数模式(接线 infer_modes)
        // r [x y] body (== y y) → x 未使用 → Free,y 使用 → In
        let r = CoreDef {
            name: Symbol::new("r"),
            ty: None,
            effects: tisp_core::types::EffectRow::Pure,
            grade: tisp_core::types::Grade::Omega,
            mode: Mode::Free,
            mode_sigs: vec![],
            determinism: tisp_core::types::Determinism::Det,
            region: None,
            visibility: Visibility::Public,
            body: e(CoreExprNode::Lam(tisp_core::core_ast::Lambda {
                params: vec![
                    tisp_core::core_ast::Param { name: Symbol::new("x"), ty: None, grade: tisp_core::types::Grade::Omega, mode: Mode::In },
                    tisp_core::core_ast::Param { name: Symbol::new("y"), ty: None, grade: tisp_core::types::Grade::Omega, mode: Mode::In },
                ],
                body: Box::new(e(var("y"))),
                ret_type: None,
            })),
            requires: None,
            ensures: None,
            span: Span::dummy(),
        };
        let prog = CoreProgram { data_decls: vec![], effect_decls: vec![], type_families: vec![], resource_algebras: vec![], defs: vec![r] , pragmas: vec![] };
        let mut m = ModeAnalyzer::new();
        assert!(m.analyze_program(&prog).is_ok());
        let inferred = m.inferred_sigs.get(&Symbol::new("r")).cloned().unwrap_or_default();
        assert!(inferred.contains(&vec![Mode::Free, Mode::In]),
            "应从函数体推断 [Free, In](x 未使用→Free,y 使用→In),实际 {:?}", inferred);
    }
}
