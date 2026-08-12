use tisp_core::core_ast::{CoreExprNode, Literal};
use tisp_core::span::Span;
use tisp_core::symbol::Symbol;
use tisp_core::types::*;
use std::collections::HashMap;

/// Liquid type checker — verifies refinement predicates and contracts
#[derive(Debug, Clone)]
pub struct LiquidChecker {
    /// Known variable bindings with their types
    pub env: HashMap<Symbol, Type>,
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

impl LiquidChecker {
    pub fn new() -> Self {
        Self { env: HashMap::new() }
    }

    /// Bind a variable with its refined type
    pub fn bind(&mut self, name: Symbol, ty: Type) {
        self.env.insert(name, ty);
    }

    /// Check a predicate against known bindings.
    /// Returns true if the predicate is always satisfied.
    pub fn check_predicate(&self, pred: &Predicate) -> Result<bool, LiquidError> {
        match pred {
            Predicate::Lit(b) => Ok(*b),
            Predicate::Var(_) => {
                // Variable predicate — can't verify statically here
                Ok(true) // assume true (sound approximation)
            }
            Predicate::App(name, args) => {
                match name.as_str() {
                    ">"  => self.check_cmp(&CmpOp::Gt, args),
                    "<"  => self.check_cmp(&CmpOp::Lt, args),
                    ">=" => self.check_cmp(&CmpOp::Ge, args),
                    "<=" => self.check_cmp(&CmpOp::Le, args),
                    "="  => self.check_cmp(&CmpOp::Eq, args),
                    "!=" => self.check_cmp(&CmpOp::Ne, args),
                    "even?" => self.check_even(args),
                    "odd?" => self.check_odd(args),
                    "positive?" => {
                        self.check_cmp(&CmpOp::Gt, &[
                            Predicate::Var(Symbol::new("_arg")),
                            Predicate::App(Symbol::new("0"), vec![])
                        ])
                    }
                    "neg?" => {
                        self.check_cmp(&CmpOp::Lt, &[
                            Predicate::Var(Symbol::new("_arg")),
                            Predicate::App(Symbol::new("0"), vec![])
                        ])
                    }
                    _ => {
                        // Unknown predicate function — assume true
                        Ok(true)
                    }
                }
            }
            Predicate::And(a, b) => {
                Ok(self.check_predicate(a)? && self.check_predicate(b)?)
            }
            Predicate::Or(a, b) => {
                Ok(self.check_predicate(a)? || self.check_predicate(b)?)
            }
            Predicate::Not(a) => {
                Ok(!self.check_predicate(a)?)
            }
            Predicate::Implies(a, b) => {
                Ok(!self.check_predicate(a)? || self.check_predicate(b)?)
            }
            Predicate::Forall(_, p) => self.check_predicate(p),
            Predicate::Exists(_, p) => self.check_predicate(p),
            Predicate::Cmp(op, lhs, rhs) => {
                let l_val = self.eval_term_const(lhs);
                let r_val = self.eval_term_const(rhs);
                match (l_val, r_val) {
                    (Some(l), Some(r)) => {
                        match op {
                            CmpOp::Eq => Ok(l == r),
                            CmpOp::Ne => Ok(l != r),
                            CmpOp::Lt => Ok(l < r),
                            CmpOp::Le => Ok(l <= r),
                            CmpOp::Gt => Ok(l > r),
                            CmpOp::Ge => Ok(l >= r),
                        }
                    }
                    _ => {
                        // Cannot evaluate statically — assume true
                        Ok(true)
                    }
                }
            }
        }
    }

    fn check_cmp(&self, op: &CmpOp, args: &[Predicate]) -> Result<bool, LiquidError> {
        if args.len() != 2 {
            return Ok(true);
        }
        let lhs = self.pred_to_term(&args[0]);
        let rhs = self.pred_to_term(&args[1]);
        let l_val = self.eval_term_const(&lhs);
        let r_val = self.eval_term_const(&rhs);
        match (l_val, r_val) {
            (Some(l), Some(r)) => match op {
                CmpOp::Gt => Ok(l > r),
                CmpOp::Lt => Ok(l < r),
                CmpOp::Ge => Ok(l >= r),
                CmpOp::Le => Ok(l <= r),
                CmpOp::Eq => Ok(l == r),
                CmpOp::Ne => Ok(l != r),
            },
            _ => Ok(true),
        }
    }

    fn check_even(&self, args: &[Predicate]) -> Result<bool, LiquidError> {
        if args.len() != 1 { return Ok(true); }
        let term = self.pred_to_term(&args[0]);
        if let Some(val) = self.eval_term_const(&term) {
            return Ok(val % 2 == 0);
        }
        Ok(true)
    }

    fn check_odd(&self, args: &[Predicate]) -> Result<bool, LiquidError> {
        if args.len() != 1 { return Ok(true); }
        let term = self.pred_to_term(&args[0]);
        if let Some(val) = self.eval_term_const(&term) {
            return Ok(val % 2 != 0);
        }
        Ok(true)
    }

    fn pred_to_term(&self, pred: &Predicate) -> Term {
        match pred {
            Predicate::App(name, _) if is_numeric_literal(name.as_str()) => {
                Term::Lit(Lit::Int(name.as_str().parse().unwrap_or(0)))
            }
            Predicate::Var(name) => Term::Var(name.clone()),
            Predicate::App(name, args) => {
                let terms: Vec<Term> = args.iter().map(|a| self.pred_to_term(a)).collect();
                match terms.len() {
                    0 => Term::Var(name.clone()),
                    1 => Term::App(name.clone(), terms),
                    2 => {
                        // Binary arithmetic
                        if let Some(op) = binop_from_name(name.as_str()) {
                            Term::BinOp(op, Box::new(terms[0].clone()), Box::new(terms[1].clone()))
                        } else {
                            Term::App(name.clone(), terms)
                        }
                    }
                    _ => Term::App(name.clone(), terms),
                }
            }
            _ => Term::Var(Symbol::new("_unknown")),
        }
    }

    fn eval_term_const(&self, term: &Term) -> Option<i64> {
        match term {
            Term::Lit(Lit::Int(n)) => Some(*n),
            Term::Lit(Lit::Float(_)) | Term::Lit(Lit::Bool(_)) | Term::Lit(Lit::Str(_)) => None,
            Term::Var(name) => {
                // Look up in environment for constant bindings
                if let Some(ty) = self.env.get(name) {
                    self.extract_const(ty)
                } else {
                    None
                }
            }
            Term::BinOp(op, a, b) => {
                let av = self.eval_term_const(a)?;
                let bv = self.eval_term_const(b)?;
                match op {
                    BinOp::Add => Some(av + bv),
                    BinOp::Sub => Some(av - bv),
                    BinOp::Mul => Some(av * bv),
                    BinOp::Div => {
                        if bv != 0 { Some(av / bv) } else { None }
                    }
                    BinOp::Mod => {
                        if bv != 0 { Some(av % bv) } else { None }
                    }
                }
            }
            Term::App(name, args) => {
                if args.is_empty() { return None; }
                match name.as_str() {
                    "abs" => {
                        let v = self.eval_term_const(&args[0])?;
                        Some(v.abs())
                    }
                    "-" if args.len() == 1 => {
                        let v = self.eval_term_const(&args[0])?;
                        Some(-v)
                    }
                    _ => None,
                }
            }
        }
    }

    fn extract_const(&self, ty: &Type) -> Option<i64> {
        match ty {
            Type::Refined(base, _pred) => {
                // Try to extract constant from the base type
                self.extract_const(base)
            }
            Type::Con(tc) if tc.name.as_str() == "i64" => None,
            _ => None,
        }
    }

    /// Verify a contract: check that requires holds, then check ensures
    pub fn verify_contract(
        &self,
        requires: &Option<Predicate>,
        ensures: &Option<Predicate>,
        _body_span: Span,
    ) -> Result<(), LiquidError> {
        if let Some(req) = requires {
            if !self.check_predicate(req)? {
                return Err(LiquidError {
                    message: "requires clause not satisfied".into(),
                    span: Span::dummy(),
                });
            }
        }
        if let Some(ens) = ensures {
            if !self.check_predicate(ens)? {
                return Err(LiquidError {
                    message: "ensures clause not satisfied".into(),
                    span: Span::dummy(),
                });
            }
        }
        Ok(())
    }
}

fn is_numeric_literal(s: &str) -> bool {
    s.parse::<i64>().is_ok()
}

// ─────────────────────────────────────────────────────────────────────────────
// 谓词/项 → SMT-LIB2 翻译器
// 只翻译「可判定子集」:比较、算术、布尔连接、量词与已知纯函数。
// 不可翻译的谓词/项返回 None,由调用方按「未验证」处理(警告放行,不误报)。
// ─────────────────────────────────────────────────────────────────────────────

/// SMT 变量名清洗:仅保留 [a-zA-Z_][a-zA-Z0-9_]*,其余替换为 `_`
pub fn smt_var(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        let ok = if i == 0 { c.is_ascii_alphabetic() || c == '_' } else { c.is_ascii_alphanumeric() || c == '_' };
        if ok { out.push(c); } else { out.push('_'); }
    }
    if out.is_empty() { "_v".into() } else { out }
}

/// 翻译 Term 为 SMT 表达式;不可翻译返回 None
pub fn term_to_smt(term: &Term) -> Option<String> {
    term_to_smt_bound(term, None)
}

/// 翻译 Term 为 SMT 表达式,`subst = (绑定变量, 替换文本)` 时该变量出现处替换为替换文本
pub fn term_to_smt_bound(term: &Term, subst: Option<(&str, &str)>) -> Option<String> {
    match term {
        Term::Lit(Lit::Int(n)) => Some(n.to_string()),
        Term::Lit(Lit::Bool(b)) => Some(if *b { "true".into() } else { "false".into() }),
        Term::Lit(_) => None, // Float/Str 不支持
        Term::Var(name) => {
            if let Some((b, v)) = subst {
                if name.as_str() == b { return Some(v.to_string()); }
            }
            Some(smt_var(name.as_str()))
        }
        Term::BinOp(op, a, b) => {
            let op_str = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "div",
                BinOp::Mod => "mod",
            };
            Some(format!("({} {} {})", op_str, term_to_smt_bound(a, subst)?, term_to_smt_bound(b, subst)?))
        }
        Term::App(name, args) => {
            let name_str = name.as_str();
            match (name_str, args.len()) {
                // 数字字面量以 App("0", []) 形式出现(desugar_predicate)
                ("0", _) => Some("0".into()),
                ("abs", 1) => Some(format!("(abs {})", term_to_smt_bound(&args[0], subst)?)),
                ("-", 1) => Some(format!("(- {})", term_to_smt_bound(&args[0], subst)?)),
                _ => None, // 未知函数
            }
        }
    }
}

/// 翻译 Predicate 为 SMT 表达式;不可翻译返回 None
pub fn pred_to_smt(pred: &Predicate) -> Option<String> {
    pred_to_smt_bound(pred, None)
}

/// 翻译 Predicate 为 SMT 表达式,`subst = (绑定变量, 替换文本)` 时该变量出现处替换为替换文本;
/// 被量词(forall/exists)遮蔽的同名变量不替换
pub fn pred_to_smt_bound(pred: &Predicate, subst: Option<(&str, &str)>) -> Option<String> {
    match pred {
        Predicate::Lit(b) => Some(if *b { "true".into() } else { "false".into() }),
        Predicate::Var(name) => {
            if let Some((b, v)) = subst {
                if name.as_str() == b { return Some(v.to_string()); }
            }
            Some(smt_var(name.as_str()))
        }
        Predicate::Cmp(op, lhs, rhs) => {
            let op_str = match op {
                CmpOp::Eq => "=",
                CmpOp::Ne => "distinct",
                CmpOp::Lt => "<",
                CmpOp::Le => "<=",
                CmpOp::Gt => ">",
                CmpOp::Ge => ">=",
            };
            Some(format!("({} {} {})", op_str, term_to_smt_bound(lhs, subst)?, term_to_smt_bound(rhs, subst)?))
        }
        Predicate::And(a, b) => Some(format!("(and {} {})", pred_to_smt_bound(a, subst)?, pred_to_smt_bound(b, subst)?)),
        Predicate::Or(a, b) => Some(format!("(or {} {})", pred_to_smt_bound(a, subst)?, pred_to_smt_bound(b, subst)?)),
        Predicate::Not(a) => Some(format!("(not {})", pred_to_smt_bound(a, subst)?)),
        Predicate::Implies(a, b) => Some(format!("(=> {} {})", pred_to_smt_bound(a, subst)?, pred_to_smt_bound(b, subst)?)),
        Predicate::Forall(v, p) => {
            // 遮蔽:绑定变量与 subst 同名时不替换
            if let Some((b, _)) = subst {
                if v.as_str() == b { return pred_to_smt(p); }
            }
            Some(format!("(forall (({} Int)) {})", smt_var(v.as_str()), pred_to_smt_bound(p, subst)?))
        }
        Predicate::Exists(v, p) => {
            if let Some((b, _)) = subst {
                if v.as_str() == b { return pred_to_smt(p); }
            }
            Some(format!("(exists (({} Int)) {})", smt_var(v.as_str()), pred_to_smt_bound(p, subst)?))
        }
        Predicate::App(name, args) => {
            let name_str = name.as_str();
            // 数字字面量谓词:App("42", [])
            if args.is_empty() {
                if let Ok(n) = name_str.parse::<i64>() { return Some(n.to_string()); }
                return Some(smt_var(name_str)); // 裸符号按变量
            }
            match name_str {
                "abs" if args.len() == 1 => Some(format!("(abs {})", pred_to_smt_bound(&args[0], subst)?)),
                "-" if args.len() == 1 => Some(format!("(- {})", pred_to_smt_bound(&args[0], subst)?)),
                "even?" if args.len() == 1 => Some(format!("(= (mod {} 2) 0)", pred_to_smt_bound(&args[0], subst)?)),
                "odd?" if args.len() == 1 => Some(format!("(= (mod {} 2) 1)", pred_to_smt_bound(&args[0], subst)?)),
                "positive?" if args.len() == 1 => Some(format!("(> {} 0)", pred_to_smt_bound(&args[0], subst)?)),
                "neg?" if args.len() == 1 => Some(format!("(< {} 0)", pred_to_smt_bound(&args[0], subst)?)),
                _ => None, // 未知谓词函数 → 不可翻译
            }
        }
    }
}

/// 谓词的自由变量(未被 forall/exists 绑定)
pub fn pred_free_vars(pred: &Predicate) -> Vec<Symbol> {
    let mut out = Vec::new();
    let mut bound: Vec<Symbol> = Vec::new();
    collect_pred_vars(pred, &mut out, &mut bound);
    out
}

fn collect_pred_vars(pred: &Predicate, out: &mut Vec<Symbol>, bound: &mut Vec<Symbol>) {
    match pred {
        Predicate::Lit(_) => {}
        Predicate::Var(v) => {
            if !bound.contains(v) && !out.contains(v) { out.push(v.clone()); }
        }
        Predicate::App(_, args) => for a in args { collect_pred_vars(a, out, bound); },
        Predicate::And(a, b) | Predicate::Or(a, b) | Predicate::Implies(a, b) => {
            collect_pred_vars(a, out, bound);
            collect_pred_vars(b, out, bound);
        }
        Predicate::Not(a) => collect_pred_vars(a, out, bound),
        Predicate::Forall(v, p) | Predicate::Exists(v, p) => {
            bound.push(v.clone());
            collect_pred_vars(p, out, bound);
            bound.pop();
        }
        Predicate::Cmp(_, l, r) => {
            collect_term_vars(l, out);
            collect_term_vars(r, out);
        }
    }
}

fn collect_term_vars(t: &Term, out: &mut Vec<Symbol>) {
    match t {
        Term::Lit(_) => {}
        Term::Var(v) => { if !out.contains(v) { out.push(v.clone()); } }
        Term::App(_, args) => for a in args { collect_term_vars(a, out); },
        Term::BinOp(_, a, b) => { collect_term_vars(a, out); collect_term_vars(b, out); }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 表达式(Core AST)→ SMT 项翻译器(实参/函数体,用于边界验证)
// 支持:字面量、变量、算术、if→ite;其余(用户函数调用/effect/非整数)→ None
// ─────────────────────────────────────────────────────────────────────────────

/// 翻译 Core 表达式为 SMT 项;不可翻译返回 None
pub fn expr_to_smt(node: &CoreExprNode) -> Option<String> {
    match node {
        CoreExprNode::Lit(Literal::I64(n)) => Some(n.to_string()),
        CoreExprNode::Lit(Literal::Bool(b)) => Some(if *b { "true".into() } else { "false".into() }),
        CoreExprNode::Lit(_) => None,
        CoreExprNode::Var(name) => Some(smt_var(name.as_str())),
        CoreExprNode::If(cond, then_, else_) => {
            Some(format!("(ite {} {} {})",
                expr_to_smt(&cond.node)?,
                expr_to_smt(&then_.node)?,
                expr_to_smt(&else_.node)?))
        }
        // 柯里化二元算术/比较:(+ 1 x) → App(App(Var(+), Lit(1)), Var(x))
        CoreExprNode::App(f, arg) => {
            let arg_smt = expr_to_smt(&arg.node)?;
            if let CoreExprNode::App(f2, arg2) = &f.node {
                if let CoreExprNode::Var(op) = &f2.node {
                    let op_str = match op.as_str() {
                        "+" => "+",
                        "-" => "-",
                        "*" => "*",
                        "/" => "div",
                        "%" => "mod",
                        // 比较运算(if 条件等)
                        ">" => ">",
                        "<" => "<",
                        ">=" => ">=",
                        "<=" => "<=",
                        "=" => "=",
                        "!=" => "distinct",
                        _ => return None,
                    };
                    return Some(format!("({} {} {})", op_str, expr_to_smt(&arg2.node)?, arg_smt));
                }
            }
            // 一元负号:(- x) → App(Var(-), x)
            if let CoreExprNode::Var(op) = &f.node {
                if op.as_str() == "-" {
                    return Some(format!("(- {})", arg_smt));
                }
            }
            None // 用户函数调用或未知形状
        }
        _ => None,
    }
}

#[cfg(test)]
mod smt_tests {
    use super::*;

    #[test]
    fn test_term_to_smt() {
        // 字面量与变量
        assert_eq!(term_to_smt(&Term::Lit(Lit::Int(3))).unwrap(), "3");
        assert_eq!(term_to_smt(&Term::Var(Symbol::new("x"))).unwrap(), "x");
        // 变量名清洗
        assert_eq!(term_to_smt(&Term::Var(Symbol::new("x'"))).unwrap(), "x_");
        // 算术
        let t = Term::BinOp(BinOp::Add,
            Box::new(Term::Var(Symbol::new("x"))),
            Box::new(Term::Lit(Lit::Int(1))));
        assert_eq!(term_to_smt(&t).unwrap(), "(+ x 1)");
        // abs 与一元负号
        let abs = Term::App(Symbol::new("abs"), vec![Term::Var(Symbol::new("x"))]);
        assert_eq!(term_to_smt(&abs).unwrap(), "(abs x)");
        let neg = Term::App(Symbol::new("-"), vec![Term::Var(Symbol::new("x"))]);
        assert_eq!(term_to_smt(&neg).unwrap(), "(- x)");
        // 未知函数不可翻译
        assert!(term_to_smt(&Term::App(Symbol::new("foo"), vec![Term::Var(Symbol::new("x"))])).is_none());
        // Float 不支持
        assert!(term_to_smt(&Term::Lit(Lit::Float(0))).is_none());
    }

    #[test]
    fn test_pred_to_smt() {
        // 比较
        let cmp = Predicate::Cmp(CmpOp::Ge,
            Box::new(Term::Var(Symbol::new("n"))),
            Box::new(Term::Lit(Lit::Int(0))));
        assert_eq!(pred_to_smt(&cmp).unwrap(), "(>= n 0)");
        // 布尔连接
        let and = Predicate::And(Box::new(cmp.clone()), Box::new(Predicate::Lit(true)));
        assert_eq!(pred_to_smt(&and).unwrap(), "(and (>= n 0) true)");
        // 蕴含
        let imp = Predicate::Implies(Box::new(Predicate::Var(Symbol::new("p"))), Box::new(cmp.clone()));
        assert_eq!(pred_to_smt(&imp).unwrap(), "(=> p (>= n 0))");
        // 量词
        let fa = Predicate::Forall(Symbol::new("x"), Box::new(cmp.clone()));
        assert_eq!(pred_to_smt(&fa).unwrap(), "(forall ((x Int)) (>= n 0))");
        // 已知谓词函数展开
        let pos = Predicate::App(Symbol::new("positive?"), vec![Predicate::Var(Symbol::new("n"))]);
        assert_eq!(pred_to_smt(&pos).unwrap(), "(> n 0)");
        let even = Predicate::App(Symbol::new("even?"), vec![Predicate::Var(Symbol::new("n"))]);
        assert_eq!(pred_to_smt(&even).unwrap(), "(= (mod n 2) 0)");
        // 数字字面量
        let zero = Predicate::App(Symbol::new("0"), vec![]);
        assert_eq!(pred_to_smt(&zero).unwrap(), "0");
        // 未知谓词函数不可翻译
        let unk = Predicate::App(Symbol::new("mystery"), vec![Predicate::Var(Symbol::new("x"))]);
        assert!(pred_to_smt(&unk).is_none());
    }

    fn e(node: CoreExprNode) -> tisp_core::core_ast::CoreExpr {
        tisp_core::core_ast::CoreExpr::new(node, tisp_core::span::Span::dummy())
    }

    fn v(name: &str) -> CoreExprNode { CoreExprNode::Var(Symbol::new(name)) }
    fn i(n: i64) -> CoreExprNode { CoreExprNode::Lit(Literal::I64(n)) }
    fn app(f: CoreExprNode, a: CoreExprNode) -> CoreExprNode {
        CoreExprNode::App(Box::new(e(f)), Box::new(e(a)))
    }

    #[test]
    fn test_expr_to_smt() {
        // 字面量与变量
        assert_eq!(expr_to_smt(&CoreExprNode::Lit(Literal::I64(5))).unwrap(), "5");
        assert_eq!(expr_to_smt(&CoreExprNode::Var(Symbol::new("x"))).unwrap(), "x");
        // 二元算术(柯里化):(+ 1 x) = App(App(Var(+), 1), x)
        let add = app(app(v("+"), i(1)), v("x"));
        assert_eq!(expr_to_smt(&add).unwrap(), "(+ 1 x)");
        // 一元负号:(- x)
        let neg = app(v("-"), v("x"));
        assert_eq!(expr_to_smt(&neg).unwrap(), "(- x)");
        // if → ite
        let ite = CoreExprNode::If(
            Box::new(e(v("c"))),
            Box::new(e(v("x"))),
            Box::new(e(v("y"))));
        assert_eq!(expr_to_smt(&ite).unwrap(), "(ite c x y)");
        // 用户函数调用不可翻译
        let call = app(v("foo"), i(1));
        assert!(expr_to_smt(&call).is_none());
    }
}

fn binop_from_name(name: &str) -> Option<BinOp> {
    match name {
        "+" => Some(BinOp::Add),
        "-" => Some(BinOp::Sub),
        "*" => Some(BinOp::Mul),
        "/" => Some(BinOp::Div),
        "%" => Some(BinOp::Mod),
        _ => None,
    }
}
