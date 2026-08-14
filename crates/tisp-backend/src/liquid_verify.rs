//! 液态类型验证驱动:LiquidVerifier
//!
//! 在类型推断完成后运行,对精化类型与契约做 SMT 求解验证:
//! 1. 调用点:实参满足参数精化谓词与函数 `:requires`
//! 2. 返回精化:函数体满足返回类型 `{x : T | pred}`
//! 3. 契约:`:requires` ⇒ `:ensures`(result 绑定为函数体)
//!
//! 验证语义(见 docs/spec.md §15):
//! - unsat(无反例)→ 通过,计入 verified
//! - sat(反例)→ 违反,计入 violated 并产出带反例的错误(span 定位)
//! - unknown / 不可翻译 → 警告放行,计入 warned(不误报)
//! - z3 二进制不可用 → 降级:全部跳过,degraded = true(保持现状行为)

use std::collections::HashMap;

use tisp_core::core_ast::{CoreDef, CoreExpr, CoreExprNode, CoreProgram};
use tisp_core::span::Span;
use tisp_core::symbol::Symbol;
use tisp_core::types::{Predicate, Type};
use tisp_middle::grade_check::GradeInequality;
use tisp_middle::liquid_types::{expr_to_smt, pred_free_vars, pred_to_smt, pred_to_smt_bound, smt_var};

use crate::z3_bridge::{VerifyOutcome, Z3Bridge, format_counterexample};

/// 液态验证报告
#[derive(Debug, Clone, Default)]
pub struct LiquidReport {
    /// 验证通过(unsat)项数
    pub verified: usize,
    /// 违反(sat 反例)项数
    pub violated: usize,
    /// 无法判定(unknown/不可翻译)警告数
    pub warned: usize,
    /// z3 不可用,降级为常量折叠
    pub degraded: bool,
    /// 违反明细(错误)
    pub errors: Vec<LiquidError>,
}

#[derive(Debug, Clone)]
pub struct LiquidError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for LiquidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "liquid type error: {} at {}", self.message, self.span)
    }
}
impl std::error::Error for LiquidError {}

pub struct LiquidVerifier {
    /// z3 桥;None 表示降级模式
    z3: Option<Z3Bridge>,
    report: LiquidReport,
    /// 函数签名表:name → 参数类型
    sigs: HashMap<String, Vec<Option<Type>>>,
    /// 定义表:name → 定义(取参数名与 requires)
    defs: HashMap<String, CoreDef>,
}

impl LiquidVerifier {
    pub fn new() -> Self {
        let z3 = match Z3Bridge::new() {
            Ok(bridge) => Some(bridge),
            Err(_) => None, // 降级:保持仅常量折叠行为
        };
        Self { z3, report: LiquidReport::default(), sigs: HashMap::new(), defs: HashMap::new() }
    }

    /// 当前报告(供多阶段验证合并)
    pub fn report(&self) -> &LiquidReport {
        &self.report
    }

    /// 是否降级(无 z3)
    pub fn degraded(&self) -> bool {
        self.z3.is_none()
    }

    /// 对程序运行全部液态验证,返回报告
    pub fn verify_program(&mut self, program: &CoreProgram) -> LiquidReport {
        self.report = LiquidReport { degraded: self.z3.is_none(), ..Default::default() };
        self.sigs.clear();
        self.defs.clear();

        // 构建签名表与定义表
        for def in &program.defs {
            if let CoreExprNode::Lam(lam) = &def.body.node {
                self.sigs.insert(def.name.as_str().into(), lam.params.iter().map(|p| p.ty.clone()).collect());
            }
            self.defs.insert(def.name.as_str().into(), def.clone());
        }

        // 降级模式:跳过全部求解验证
        if self.z3.is_none() {
            return self.report.clone();
        }

        for def in &program.defs {
            self.verify_def(def);
        }
        self.report.clone()
    }

    /// §10 符号等级诊断:count ≤ n 在自由等级变量 n 下无法静态判定(任何 count 都有 n=0 反例),
    /// 记录诊断性警告(含使用次数);复合等级的可折叠部分由 grade_check 常量检查处理
    pub fn verify_grade_inequalities(&mut self, ineqs: &[GradeInequality]) {
        for ineq in ineqs {
            self.report.warned += 1;
            let _ = ineq;
        }
    }

    fn verify_def(&mut self, def: &CoreDef) {
        // 返回精化类型验证
        if let Some(Type::Refined(_, pred)) = &def.ty {
            self.verify_return_refinement(def, pred);
        }
        // 契约验证:requires ⇒ ensures(result := body)
        if def.ensures.is_some() {
            self.verify_contract(def);
        }
        // 调用点验证:实参 ⇒ 参数精化 / requires
        let mut calls = Vec::new();
        collect_calls(&def.body, &mut calls);
        for (callee, args) in calls {
            self.verify_call_site(def, &callee, &args);
        }
    }

    /// 返回精化:验证函数体 ⇒ pred[自由变量 → body]
    fn verify_return_refinement(&mut self, def: &CoreDef, pred: &Predicate) {
        let Some(body) = self.lam_body(def) else { return };
        let Some(body_smt) = expr_to_smt(&body.node) else {
            self.warn("返回精化:函数体不可翻译为 SMT,跳过验证");
            return;
        };
        let fvs = pred_free_vars(pred);
        if fvs.len() != 1 {
            self.warn("返回精化:谓词自由变量个数不为 1,跳过验证");
            return;
        }
        let conclusion = match pred_to_smt_bound(pred, Some((fvs[0].as_str(), &body_smt))) {
            Some(s) => s,
            None => { self.warn("返回精化:谓词不可翻译,跳过验证"); return; }
        };
        self.check_conclusion(def.span.clone(), &format!("返回值不满足精化类型 {}", conclusion), &[], &conclusion);
    }

    /// 契约验证:requires ⇒ ensures(result := 函数体)
    fn verify_contract(&mut self, def: &CoreDef) {
        let Some(ensures) = &def.ensures else { return };
        let Some(body) = self.lam_body(def) else { return };
        let Some(body_smt) = expr_to_smt(&body.node) else {
            self.warn("契约:函数体不可翻译为 SMT,跳过 ensures 验证");
            return;
        };
        let conclusion = match pred_to_smt_bound(ensures, Some(("result", &body_smt))) {
            Some(s) => s,
            None => { self.warn("契约:ensures 不可翻译,跳过验证"); return; }
        };
        let premises: Vec<String> = def.requires.as_ref()
            .and_then(pred_to_smt)
            .into_iter()
            .collect();
        self.check_conclusion(def.span.clone(), &format!("ensures 未满足: {}", conclusion), &premises, &conclusion);
    }

    /// 解包 def 的函数体(def.body 是 Lam 包装)
    fn lam_body<'a>(&self, def: &'a CoreDef) -> Option<&'a CoreExpr> {
        if let CoreExprNode::Lam(lam) = &def.body.node {
            Some(&lam.body)
        } else {
            None
        }
    }

    /// 调用点验证:实参绑定 ⇒ 参数精化与 :requires
    fn verify_call_site(&mut self, caller: &CoreDef, callee: &Symbol, args: &[CoreExpr]) {
        let Some(param_tys) = self.sigs.get(callee.as_str()).cloned() else {
            return; // 内置函数或未知:无签名可查
        };
        // 调用点 span:取最后一个实参的 span 作为定位
        let span = args.last().map(|a| a.span.clone()).unwrap_or_else(|| caller.span.clone());

        // 实参绑定前提:(= param_j arg_j)(实参可翻译时)
        let mut premises: Vec<String> = Vec::new();
        for (j, arg) in args.iter().enumerate() {
            if j >= param_tys.len() { break; }
            let Some(arg_smt) = expr_to_smt(&arg.node) else {
                self.warn_at(span.clone(), "调用实参不可翻译为 SMT,跳过该调用点验证");
                return;
            };
            // 参数名:签名表只存了类型,参数名需要从 callee 定义取
            if let Some(param) = self.param_name(callee, j) {
                premises.push(format!("(= {} {})", smt_var(&param), arg_smt));
            }
        }

        // 参数精化验证:实参绑定 ⇒ 精化谓词(自由变量替换为参数变量)
        for (j, ty) in param_tys.iter().enumerate() {
            if j >= args.len() { break; }
            let Some(Type::Refined(_, pred)) = ty else { continue };
            let Some(param_name) = self.param_name(callee, j) else { continue };
            let fvs = pred_free_vars(pred);
            if fvs.len() != 1 {
                self.warn_at(span.clone(), "参数精化:谓词自由变量个数不为 1,跳过验证");
                continue;
            }
            let conclusion = match pred_to_smt_bound(pred, Some((fvs[0].as_str(), &smt_var(&param_name)))) {
                Some(s) => s,
                None => { self.warn_at(span.clone(), "参数精化:谓词不可翻译,跳过验证"); continue; }
            };
            self.check_conclusion(span.clone(), &format!("实参不满足参数精化 {}", conclusion), &premises, &conclusion);
        }

        // 契约 requires 验证:实参绑定 ⇒ requires
        if let Some(requires) = self.requires_of(callee) {
            let Some(conclusion) = pred_to_smt(&requires) else {
                self.warn_at(span.clone(), "requires 不可翻译,跳过该调用点验证");
                return;
            };
            self.check_conclusion(span.clone(), &format!("调用违反契约 requires: {}", conclusion), &premises, &conclusion);
        }
    }

    /// 取 callee 定义的第 j 个参数名
    fn param_name(&self, callee: &Symbol, j: usize) -> Option<String> {
        self.def_of(callee).map(|def| {
            if let CoreExprNode::Lam(lam) = &def.body.node {
                lam.params.get(j).map(|p| p.name.as_str().to_string())
            } else {
                None
            }
        }).flatten()
    }

    fn requires_of(&self, callee: &Symbol) -> Option<Predicate> {
        self.def_of(callee).and_then(|def| def.requires.clone())
    }

    fn def_of(&self, name: &Symbol) -> Option<&CoreDef> {
        self.defs.get(name.as_str())
    }

    /// 执行蕴含验证并记录结果
    fn check_conclusion(&mut self, span: Span, label: &str, premises: &[String], conclusion: &str) {
        let Some(z3) = &mut self.z3 else { return };
        match z3.verify_implication(premises, conclusion) {
            Ok(VerifyOutcome::Unsat) => self.report.verified += 1,
            Ok(VerifyOutcome::Sat(model)) => {
                self.report.violated += 1;
                let ce = format_counterexample(&model);
                self.report.errors.push(LiquidError {
                    message: format!("{};反例: {}", label, ce),
                    span,
                });
            }
            Ok(VerifyOutcome::Unknown) | Err(_) => self.report.warned += 1,
        }
    }

    fn warn(&mut self, _msg: &str) {
        self.report.warned += 1;
    }

    fn warn_at(&mut self, _span: Span, _msg: &str) {
        self.report.warned += 1;
    }
}

/// 收集表达式中所有直接调用:(callee, args)
fn collect_calls(expr: &CoreExpr, out: &mut Vec<(Symbol, Vec<CoreExpr>)>) {
    match &expr.node {
        CoreExprNode::App(_, _) => {
            // 展开完整柯里化调用链,恰好收集一次(避免把链内部 App 重复收集)
            if let Some((name, args)) = collect_call_chain(expr) {
                out.push((name, args.clone()));
                // 递归每个实参,收集嵌套调用
                for a in &args {
                    collect_calls(a, out);
                }
            }
        }
        CoreExprNode::If(c, t, e) => { collect_calls(c, out); collect_calls(t, out); collect_calls(e, out); }
        CoreExprNode::Let(_, _, v, body) => { collect_calls(v, out); collect_calls(body, out); }
        CoreExprNode::Do(items) => { for i in items { collect_calls(i, out); } }
        CoreExprNode::Lam(lam) => collect_calls(&lam.body, out),
        CoreExprNode::Match(s, arms) => {
            collect_calls(s, out);
            for arm in arms { collect_calls(&arm.body, out); }
        }
        CoreExprNode::Handle(h, _) => collect_calls(h, out),
        CoreExprNode::Data(_, args) => { for a in args { collect_calls(a, out); } }
        CoreExprNode::Perform(_, args) => { for a in args { collect_calls(a, out); } }
        _ => {}
    }
}

/// 收集 (callee, args) 调用链:App(App(App(Var(f), a1), a2), a3) → (f, [a1, a2, a3])
fn collect_call_chain(expr: &CoreExpr) -> Option<(Symbol, Vec<CoreExpr>)> {
    match &expr.node {
        CoreExprNode::Var(name) => Some((name.clone(), Vec::new())),
        CoreExprNode::App(f, arg) => {
            collect_call_chain(f).map(|(name, mut args)| {
                args.push((**arg).clone());
                (name, args)
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tisp_frontend::desugar::Desugarer;
    use tisp_frontend::reader::read;

    fn program(src: &str) -> CoreProgram {
        let forms = read(src).unwrap();
        Desugarer::new().desugar_program(forms).unwrap()
    }

    fn verify(src: &str) -> LiquidReport {
        let prog = program(src);
        let mut v = LiquidVerifier::new();
        v.verify_program(&prog)
    }

    fn errors_text(report: &LiquidReport) -> String {
        report.errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>().join("; ")
    }

    #[test]
    fn test_return_refinement_ok() {
        // §15.2:两分支均满足返回精化(if → ite 路径敏感)
        let src = r#"
(defn abs [x] -> {n : i64 | (>= n 0)} (if (>= x 0) x (- x)))
(defn main [] (abs 5))
"#;
        let r = verify(src);
        assert_eq!(r.violated, 0, "不应有违反:{}", errors_text(&r));
        assert!(r.verified >= 1, "应有返回精化验证通过");
    }

    #[test]
    fn test_return_refinement_violation() {
        // 直接返回参数:可负 → 违反,带反例
        let src = r#"
(defn bad [x] -> {n : i64 | (>= n 0)} x)
(defn main [] (bad 5))
"#;
        let r = verify(src);
        assert!(r.violated >= 1, "应报告返回精化违反:{}", errors_text(&r));
        let text = errors_text(&r);
        assert!(text.contains("反例"), "错误应带反例:{}", text);
    }

    #[test]
    fn test_call_site_refinement_violation() {
        // 调用违反参数精化:(sqrt -1) → 反例 x = -1
        let src = r#"
(defn sqrt [x : {n : i64 | (>= n 0)}] -> i64 x)
(defn main [] (sqrt -1))
"#;
        let r = verify(src);
        assert!(r.violated >= 1, "应报告参数精化违反:{}", errors_text(&r));
        let text = errors_text(&r);
        assert!(text.contains("x = -1"), "反例应为 x = -1:{}", text);
    }

    #[test]
    fn test_call_site_refinement_ok() {
        let src = r#"
(defn sqrt [x : {n : i64 | (>= n 0)}] -> i64 x)
(defn main [] (sqrt 9))
"#;
        let r = verify(src);
        assert_eq!(r.violated, 0, "合法调用不应违反:{}", errors_text(&r));
    }

    #[test]
    fn test_requires_violation() {
        // 调用违反契约 requires:(divide 1 0) → 反例 d = 0
        let src = r#"
(defn divide [n d] :requires (!= d 0) n)
(defn main [] (divide 1 0))
"#;
        let r = verify(src);
        assert!(r.violated >= 1, "应报告契约违反:{}", errors_text(&r));
        let text = errors_text(&r);
        assert!(text.contains("requires"), "错误应提及 requires:{}", text);
        assert!(text.contains("d = 0"), "反例应为 d = 0:{}", text);
    }

    #[test]
    fn test_contract_ok() {
        // requires ⇒ ensures 满足:两个 requires 合取 + 调用点合法
        let src = r#"
(defn add-pos [x y] :requires (> x 0) :requires (> y 0) :ensures (> result 0) (+ x y))
(defn main [] (add-pos 1 2))
"#;
        let r = verify(src);
        assert_eq!(r.violated, 0, "契约应满足:{}", errors_text(&r));
        assert!(r.verified >= 2, "契约与调用点均应验证通过");
    }

    #[test]
    fn test_ensures_violation() {
        // 无 requires 的 ensures:可违反
        let src = r#"
(defn add-pos [x y] :ensures (> result 0) (+ x y))
(defn main [] (add-pos 1 2))
"#;
        let r = verify(src);
        assert!(r.violated >= 1, "应报告 ensures 违反:{}", errors_text(&r));
    }

    #[test]
    fn test_untranslatable_warns_not_violates() {
        // 未知谓词函数:警告放行,不误报
        let src = r#"
(defn weird [x : {n : i64 | (mystery n)}] -> i64 x)
(defn main [] (weird 1))
"#;
        let r = verify(src);
        assert_eq!(r.violated, 0, "不可翻译谓词不应报违反:{}", errors_text(&r));
        assert!(r.warned >= 1, "应产生警告计数");
    }
}
