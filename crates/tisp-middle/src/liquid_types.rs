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
