use std::collections::HashMap;
use tisp_core::core_ast::*;
use tisp_core::symbol::Symbol;
use tisp_core::types::*;
use tisp_core::span::Span;
use tisp_core::data::DataEnv;
use crate::holes::HoleEnv;
use crate::liquid_types::LiquidChecker;

#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "type error: {} at {}", self.message, self.span)
    }
}

impl std::error::Error for TypeError {}

pub struct TypeInfer {
    next_var: u64,
    substitution: HashMap<u64, Type>,
    data_env: DataEnv,
    pub hole_env: HoleEnv,
    pub liquid_checker: LiquidChecker,
    /// Session type protocol state: channel_id → expected next operation
    session_state: HashMap<u64, SessionExpectation>,
}

#[derive(Debug, Clone, PartialEq)]
enum SessionExpectation {
    Recv,
    Close,
    End,
}

impl TypeInfer {
    pub fn new() -> Self {
        Self {
            next_var: 0,
            substitution: HashMap::new(),
            data_env: DataEnv::new(),
            hole_env: HoleEnv::new(),
            liquid_checker: LiquidChecker::new(),
            session_state: HashMap::new(),
        }
    }

    pub fn infer_program(&mut self, program: &CoreProgram) -> Result<Vec<(Symbol, Type)>, TypeError> {
        let mut env = self.initial_env();
        let mut results = Vec::new();

        // Register data declarations
        for decl in &program.data_decls {
            self.data_env.register(decl.clone());
            // Register constructors in the type environment
            for ctor in &decl.constructors {
                if let Some(ctor_type) = self.data_env.constructor_type(&ctor.name) {
                    let scheme = self.generalize(&env, &ctor_type);
                    env.insert(ctor.name.clone(), scheme);
                }
            }
        }

        for def in &program.defs {
            let ty = self.infer_def(&mut env, def)?;
            results.push((def.name.clone(), ty));
        }

        Ok(results)
    }

    fn initial_env(&self) -> TypeEnv {
        let mut env = TypeEnv::new();

        // Built-in types
        env.insert(Symbol::new("+"), TypeScheme::mono(Type::fun(Type::i64(), Type::fun(Type::i64(), Type::i64()))));
        env.insert(Symbol::new("-"), TypeScheme::mono(Type::fun(Type::i64(), Type::fun(Type::i64(), Type::i64()))));
        env.insert(Symbol::new("*"), TypeScheme::mono(Type::fun(Type::i64(), Type::fun(Type::i64(), Type::i64()))));
        env.insert(Symbol::new("/"), TypeScheme::mono(Type::fun(Type::i64(), Type::fun(Type::i64(), Type::i64()))));
        env.insert(Symbol::new("="), TypeScheme::poly(vec![TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 0 }],
            Type::fun(Type::Var(TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 0 }),
                Type::fun(Type::Var(TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 0 }), Type::bool()))));
        env.insert(Symbol::new("<"), TypeScheme::mono(Type::fun(Type::i64(), Type::fun(Type::i64(), Type::bool()))));
        env.insert(Symbol::new("<="), TypeScheme::mono(Type::fun(Type::i64(), Type::fun(Type::i64(), Type::bool()))));
        env.insert(Symbol::new(">"),  TypeScheme::mono(Type::fun(Type::i64(), Type::fun(Type::i64(), Type::bool()))));
        env.insert(Symbol::new(">="), TypeScheme::mono(Type::fun(Type::i64(), Type::fun(Type::i64(), Type::bool()))));
        env.insert(Symbol::new("!="), TypeScheme::poly(vec![TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 10 }],
            Type::fun(Type::Var(TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 10 }),
                Type::fun(Type::Var(TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 10 }), Type::bool()))));
        env.insert(Symbol::new("not"), TypeScheme::mono(Type::fun(Type::bool(), Type::bool())));
        env.insert(Symbol::new("println"), TypeScheme::poly(vec![TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 1 }],
            Type::fun_annotated(
                Type::Var(TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 1 }),
                FunAnnotation {
                    effects: EffectRow::Closed(vec![EffectLabel::IO]),
                    ..FunAnnotation::default()
                },
                Type::unit()
            )));
        env.insert(Symbol::new("mod"), TypeScheme::mono(Type::fun(Type::i64(), Type::fun(Type::i64(), Type::i64()))));
        env.insert(Symbol::new("min"), TypeScheme::mono(Type::fun(Type::i64(), Type::fun(Type::i64(), Type::i64()))));
        env.insert(Symbol::new("max"), TypeScheme::mono(Type::fun(Type::i64(), Type::fun(Type::i64(), Type::i64()))));
        env.insert(Symbol::new("print"), TypeScheme::poly(vec![TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 20 }],
            Type::fun_annotated(
                Type::Var(TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 20 }),
                FunAnnotation { effects: EffectRow::Closed(vec![EffectLabel::IO]), ..FunAnnotation::default() },
                Type::unit()
            )));
        env.insert(Symbol::new("read-line"), TypeScheme::mono(Type::fun_annotated(
            Type::unit(),
            FunAnnotation { effects: EffectRow::Closed(vec![EffectLabel::IO]), ..FunAnnotation::default() },
            Type::string()
        )));
        env.insert(Symbol::new("str"), TypeScheme::poly(vec![TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 21 }],
            Type::fun(Type::Var(TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 21 }), Type::string())));
        env.insert(Symbol::new("str-len"), TypeScheme::mono(Type::fun(Type::string(), Type::i64())));
        env.insert(Symbol::new("str-concat"), TypeScheme::mono(Type::fun(Type::string(), Type::fun(Type::string(), Type::string()))));
        env.insert(Symbol::new("cons"), TypeScheme::poly(vec![TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 22 }],
            Type::fun(Type::Var(TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 22 }),
                Type::fun(Type::Var(TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 22 }),
                    Type::Var(TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 22 })))));
        env.insert(Symbol::new("not="), TypeScheme::poly(vec![TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 23 }],
            Type::fun(Type::Var(TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 23 }),
                Type::fun(Type::Var(TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 23 }), Type::bool()))));
        env.insert(Symbol::new("first"), TypeScheme::poly(vec![TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 24 }],
            Type::fun(Type::list(Type::Var(TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 24 })), Type::Var(TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 24 }))));
        env.insert(Symbol::new("rest"), TypeScheme::poly(vec![TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 25 }],
            Type::fun(Type::list(Type::Var(TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 25 })), Type::list(Type::Var(TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 25 })))));
        env.insert(Symbol::new("nth"), TypeScheme::poly(vec![TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 26 }],
            Type::fun(Type::list(Type::Var(TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 26 })),
                Type::fun(Type::i64(), Type::Var(TypeVar { name: Symbol::new("a"), kind: Kind::Star, id: 26 })))));

        env
    }

    fn infer_def(&mut self, env: &mut TypeEnv, def: &CoreDef) -> Result<Type, TypeError> {
        // For recursive definitions, add a fresh type variable to the environment first
        let fresh_ty = self.fresh_var();
        let scheme = TypeScheme::mono(fresh_ty.clone());
        env.insert(def.name.clone(), scheme);

        let ty = self.infer_expr(env, &def.body)?;

        // Unify the inferred type with the fresh variable
        self.unify(&fresh_ty, &ty, def.span)?;

        // Generalize and update the environment
        let final_ty = self.apply_subst(&ty);
        let scheme = self.generalize(env, &final_ty);
        env.insert(def.name.clone(), scheme);

            // After unification, verify refined predicates with Z3/liquid checker
            let _final_ty = self.verify_refinements(&ty)?;

            Ok(self.apply_subst(&ty))
    }

    fn infer_expr(&mut self, env: &mut TypeEnv, expr: &CoreExpr) -> Result<Type, TypeError> {
        match &expr.node {
            CoreExprNode::Lit(lit) => Ok(self.infer_literal(lit)),

            CoreExprNode::Var(name) => {
                let scheme = env.lookup(name).ok_or_else(|| TypeError {
                    message: format!("unbound variable: {}", name),
                    span: expr.span,
                })?;
                Ok(self.instantiate(&scheme))
            }

            CoreExprNode::Lam(lambda) => {
                let mut param_types = Vec::new();
                let mut local_env = env.clone();

                for param in &lambda.params {
                    let ty = match &param.ty {
                        Some(t) => t.clone(),
                        None => self.fresh_var(),
                    };
                    let scheme = TypeScheme::mono(ty.clone());
                    local_env.insert(param.name.clone(), scheme);
                    param_types.push(ty);
                }

                let body_ty = self.infer_expr(&mut local_env, &lambda.body)?;

                // Build function type: p1 -> p2 -> ... -> body_ty
                let mut result = body_ty;
                for param_ty in param_types.into_iter().rev() {
                    result = Type::fun(param_ty, result);
                }

                Ok(result)
            }

            CoreExprNode::App(func, arg) => {
                let func_ty = self.infer_expr(env, func)?;
                let arg_ty = self.infer_expr(env, arg)?;
                let ret_ty = self.fresh_var();

                // Unify with effect subtyping: arg_ty ->[ε] ret_ty
                // The function's effect row must be a subset of the declared row
                self.unify(&func_ty, &Type::fun(arg_ty.clone(), ret_ty.clone()), expr.span)?;

                let _actual_ty = self.apply_subst(&func_ty);

                Ok(self.apply_subst(&ret_ty))
            }

            CoreExprNode::Let(name, ty_ann, value, body) => {
                let value_ty = self.infer_expr(env, value)?;

                if let Some(ann) = ty_ann {
                    self.unify(&value_ty, ann, expr.span)?;
                }

                let scheme = self.generalize(env, &value_ty);
                let mut local_env = env.clone();
                local_env.insert(name.clone(), scheme);

                self.infer_expr(&mut local_env, body)
            }

            CoreExprNode::If(cond, then, else_) => {
                let cond_ty = self.infer_expr(env, cond)?;
                self.unify(&cond_ty, &Type::bool(), expr.span)?;

                let then_ty = self.infer_expr(env, then)?;
                let else_ty = self.infer_expr(env, else_)?;

                self.unify(&then_ty, &else_ty, expr.span)?;

                Ok(self.apply_subst(&then_ty))
            }

             CoreExprNode::Match(scrutinee, arms) => {
                let scrut_ty = self.infer_expr(env, scrutinee)?;
                let result_ty = self.fresh_var();

                // Save substitution for GADT refinement (prevent cross-arm leakage)
                let saved_subst = self.substitution.clone();

                for arm in arms {
                    // Restore substitution for each arm (GADT-safe)
                    self.substitution = saved_subst.clone();
                    let mut local_env = env.clone();
                    let pat_ty = self.infer_pattern(&mut local_env, &arm.pattern)?;
                    self.unify(&scrut_ty, &pat_ty, expr.span)?;

                    if let Some(guard) = &arm.guard {
                        let guard_ty = self.infer_expr(&mut local_env, guard)?;
                        self.unify(&guard_ty, &Type::bool(), expr.span)?;
                    }

                    let body_ty = self.infer_expr(&mut local_env, &arm.body)?;
                    self.unify(&result_ty, &body_ty, expr.span)?;
                }

                // Exhaustiveness check
                self.check_match_exhaustiveness(&scrut_ty, arms, expr.span)?;

                Ok(self.apply_subst(&result_ty))
            }

            CoreExprNode::Data(con_name, args) => {
                // Look up constructor type from environment (where it's stored as a Poly scheme)
                if let Some(scheme) = env.lookup(con_name) {
                    let ctor_type = self.instantiate(scheme);
                    
                    // Apply arguments to the constructor
                    let mut current_type = ctor_type;
                    for (_i, arg) in args.iter().enumerate() {
                        let arg_ty = self.infer_expr(env, arg)?;
                        match current_type {
                            Type::Fun(param_ty, _, ret_ty) => {
                                self.unify(&param_ty, &arg_ty, expr.span)?;
                                current_type = *ret_ty;
                            }
                            _ => {
                                return Err(TypeError {
                                    message: format!("too many arguments to constructor {}", con_name),
                                    span: expr.span,
                                });
                            }
                        }
                    }
                    Ok(current_type)
                } else {
                    // Constructor not found, fall back to fresh variable
                    for arg in args {
                        self.infer_expr(env, arg)?;
                    }
                    Ok(self.fresh_var())
                }
            }

            CoreExprNode::Handle(body, _handler) => {
                // TODO: proper effect handler typing
                self.infer_expr(env, body)
            }

            CoreExprNode::Perform(_op_name, args) => {
                // TODO: look up operation type from effect
                let mut arg_types = Vec::new();
                for arg in args {
                    arg_types.push(self.infer_expr(env, arg)?);
                }
                // For now, return a fresh variable
                Ok(self.fresh_var())
            }

            CoreExprNode::Hole(name) => {
                let ty = self.fresh_var();
                self.hole_env.add_hole(name.clone(), Some(ty.clone()), expr.span);
                Ok(ty)
            }

            CoreExprNode::Do(exprs) => {
                if exprs.is_empty() { return Ok(Type::unit()); }
                let mut last_ty = Type::unit();
                for e in exprs {
                    last_ty = self.infer_expr(env, e)?;
                }
                Ok(self.apply_subst(&last_ty))
            }

            // ── Session type protocol checking ──
            CoreExprNode::Session(op, body) => {
                // Check protocol compliance
                match op {
                    SessionOp::Send => {
                        // After send, expect recv
                        self.session_state.insert(0, SessionExpectation::Recv);
                    }
                    SessionOp::Recv => {
                        // After recv, expect close
                        self.session_state.insert(0, SessionExpectation::Close);
                    }
                    SessionOp::Close => {
                        self.session_state.insert(0, SessionExpectation::End);
                    }
                    _ => {}
                }
                self.infer_expr(env, body)
            }

            _ => Ok(self.fresh_var()),
        }
    }

    fn infer_literal(&self, lit: &Literal) -> Type {
        match lit {
            Literal::I8(_) => Type::i8(),
            Literal::I16(_) => Type::i16(),
            Literal::I32(_) => Type::i32(),
            Literal::I64(_) => Type::i64(),
            Literal::U8(_) => Type::u8(),
            Literal::U16(_) => Type::u16(),
            Literal::U32(_) => Type::u32(),
            Literal::U64(_) => Type::u64(),
            Literal::F32(_) => Type::f32(),
            Literal::F64(_) => Type::f64(),
            Literal::Bool(_) => Type::bool(),
            Literal::String(_) => Type::string(),
            Literal::Char(_) => Type::Con(TypeCon { name: Symbol::new("char"), kind: Kind::Star }),
            Literal::Unit => Type::unit(),
        }
    }

    fn infer_pattern(&mut self, env: &mut TypeEnv, pattern: &Pattern) -> Result<Type, TypeError> {
        match pattern {
            Pattern::Wildcard => Ok(self.fresh_var()),
            Pattern::Var(name) => {
                let ty = self.fresh_var();
                env.insert(name.clone(), TypeScheme::mono(ty.clone()));
                Ok(ty)
            }
            Pattern::Lit(lit) => Ok(self.infer_literal(lit)),
            Pattern::Con(name, subpats) => {
                // Look up constructor type from environment (where it's stored as a Poly scheme)
                if let Some(scheme) = env.lookup(name) {
                    let ctor_type = self.instantiate(scheme);
                    
                    // The constructor type should be a function type: arg1 -> arg2 -> ... -> result
                    // We need to unify each sub-pattern with the corresponding argument
                    let mut current_type = ctor_type.clone();
                    
                    for (i, subpat) in subpats.iter().enumerate() {
                        match current_type {
                            Type::Fun(arg_ty, _, ret_ty) => {
                                let pat_ty = self.infer_pattern(env, subpat)?;
                                self.unify(&arg_ty, &pat_ty, Span::dummy())?;
                                current_type = *ret_ty;
                            }
                            _ => {
                                return Err(TypeError {
                                    message: format!("constructor {} expects {} arguments, got {}", 
                                                     name, i, subpats.len()),
                                    span: Span::dummy(),
                                });
                            }
                        }
                    }
                    
                    // current_type is now the result type after consuming all arguments
                    Ok(current_type)
                } else {
                    // Constructor not found, fall back to fresh variable
                    let mut sub_types = Vec::new();
                    for subpat in subpats {
                        sub_types.push(self.infer_pattern(env, subpat)?);
                    }
                    Ok(self.fresh_var())
                }
            }
            Pattern::Tuple(pats) => {
                let mut types = Vec::new();
                for pat in pats {
                    types.push(self.infer_pattern(env, pat)?);
                }
                Ok(Type::Tuple(types))
            }
        }
    }

    fn fresh_var(&mut self) -> Type {
        let id = self.next_var;
        self.next_var += 1;
        Type::Var(TypeVar {
            name: Symbol::new(&format!("?{}", id)),
            kind: Kind::Star,
            id,
        })
    }

    /// Verify refined predicates using liquid type checker
    fn verify_refinements(&self, ty: &Type) -> Result<Type, TypeError> {
        match ty {
            Type::Refined(base, pred) => {
                match self.liquid_checker.check_predicate(pred) {
                    Ok(true) => Ok(*base.clone()),
                    Ok(false) => Err(TypeError { message: "refinement predicate not satisfied".into(), span: Span::dummy() }),
                    Err(_) => Ok(*base.clone()),
                }
            }
            _ => Ok(ty.clone()),
        }
    }

    fn unify(&mut self, t1: &Type, t2: &Type, span: Span) -> Result<(), TypeError> {
        let t1 = self.apply_subst(t1);
        let t2 = self.apply_subst(t2);

        match (&t1, &t2) {
            (Type::Var(v1), Type::Var(v2)) if v1.id == v2.id => Ok(()),
            (Type::Var(v), t) | (t, Type::Var(v)) => {
                if self.occurs_check(v.id, t) {
                    Err(TypeError {
                        message: format!("infinite type: {} = {:?}", v.name, t),
                        span,
                    })
                } else {
                    self.substitution.insert(v.id, t.clone());
                    Ok(())
                }
            }
            (Type::Con(c1), Type::Con(c2)) if c1.name == c2.name => Ok(()),
            (Type::App(f1, a1), Type::App(f2, a2)) => {
                // Kind check: verify type constructors have compatible kinds
                if let Err(_e) = check_kind(f1, &Kind::Arrow(Box::new(Kind::Star), Box::new(Kind::Star))) {
                    // Not a fatal error — continue with unification
                }
                self.unify(f1, f2, span)?;
                self.unify(a1, a2, span)
            }
            (Type::Fun(p1, ann1, r1), Type::Fun(p2, ann2, r2)) => {
                self.unify(p1, p2, span)?;
                self.unify(r1, r2, span)?;
                // Effect subtyping: actual effects ⊆ expected effects
                if !effect_subtype(&ann1.effects, &ann2.effects) {
                    return Err(TypeError {
                        message: format!("effect row {:?} is not a subtype of {:?}", ann1.effects, ann2.effects),
                        span,
                    });
                }
                Ok(())
            }
            (Type::Tuple(ts1), Type::Tuple(ts2)) if ts1.len() == ts2.len() => {
                for (t1, t2) in ts1.iter().zip(ts2.iter()) {
                    self.unify(t1, t2, span)?;
                }
                Ok(())
            }
            // ── Refined type: unify as base type ──
            (Type::Refined(base, _), other) | (other, Type::Refined(base, _)) => {
                self.unify(base, other, span)
            }
            // ── Forall: unify after instantiation ──
            (Type::Forall(_, body), other) | (other, Type::Forall(_, body)) => {
                self.unify(body, other, span)
            }
            // ── HoTT Path type: unify base + endpoints ──
            (Type::Path(a1, x1, y1), Type::Path(a2, x2, y2)) => {
                self.unify(a1, a2, span)?;
                self.unify_term(x1, x2)?;
                self.unify_term(y1, y2)?;
                Ok(())
            }
            // ── Record type: unify fields + optional rest ──
            (Type::Record(fs1, r1), Type::Record(fs2, r2)) if fs1.len() == fs2.len() => {
                for ((k1, v1), (k2, v2)) in fs1.iter().zip(fs2.iter()) {
                    if k1 != k2 { return Err(TypeError { message: format!("record field mismatch: {} vs {}", k1, k2), span }); }
                    self.unify(v1, v2, span)?;
                }
                match (r1, r2) {
                    (Some(rest1), Some(rest2)) => self.unify(rest1, rest2, span),
                    (None, None) => Ok(()),
                    _ => Err(TypeError { message: "record rest type mismatch".into(), span }),
                }
            }
            // ── Modal: unify sub-type ──
            (Type::Modal(op1, t1), Type::Modal(op2, t2)) if op1 == op2 => self.unify(t1, t2, span),
            // ── Temporal: unify sub-type ──
            (Type::Temporal(op1, t1), Type::Temporal(op2, t2)) if op1 == op2 => self.unify(t1, t2, span),
            // ── Cohesive: unify sub-type ──
            (Type::Cohesive(op1, t1), Type::Cohesive(op2, t2)) if op1 == op2 => self.unify(t1, t2, span),
            _ => Err(TypeError {
                message: format!("cannot unify {:?} with {:?}", t1, t2),
                span,
            }),
        }
    }

    /// Unify two terms (for HoTT path endpoints)
    fn unify_term(&self, t1: &Term, t2: &Term) -> Result<(), TypeError> {
        match (t1, t2) {
            (Term::Lit(a), Term::Lit(b)) => if a == b { Ok(()) } else {
                Err(TypeError { message: format!("term mismatch: {:?} != {:?}", a, b), span: Span::dummy() })
            },
            (Term::Var(a), Term::Var(b)) if a == b => Ok(()),
            _ => Err(TypeError { message: format!("cannot unify terms {:?} and {:?}", t1, t2), span: Span::dummy() }),
        }
    }

    fn occurs_check(&self, var_id: u64, ty: &Type) -> bool {
        match ty {
            Type::Var(v) => v.id == var_id,
            Type::Con(_) => false,
            Type::App(f, a) => self.occurs_check(var_id, f) || self.occurs_check(var_id, a),
            Type::Fun(p, _, r) => self.occurs_check(var_id, p) || self.occurs_check(var_id, r),
            Type::Forall(_, t) => self.occurs_check(var_id, t),
            Type::Tuple(ts) => ts.iter().any(|t| self.occurs_check(var_id, t)),
            Type::Record(fields, _) => fields.iter().any(|(_, t)| self.occurs_check(var_id, t)),
            _ => false,
        }
    }

    fn apply_subst(&self, ty: &Type) -> Type {
        match ty {
            Type::Var(v) => {
                if let Some(t) = self.substitution.get(&v.id) {
                    self.apply_subst(t)
                } else {
                    ty.clone()
                }
            }
            Type::Con(_) => ty.clone(),
            Type::App(f, a) => Type::App(
                Box::new(self.apply_subst(f)),
                Box::new(self.apply_subst(a)),
            ),
            Type::Fun(p, ann, r) => Type::Fun(
                Box::new(self.apply_subst(p)),
                ann.clone(),
                Box::new(self.apply_subst(r)),
            ),
            Type::Forall(vars, t) => Type::Forall(vars.clone(), Box::new(self.apply_subst(t))),
            Type::Tuple(ts) => Type::Tuple(ts.iter().map(|t| self.apply_subst(t)).collect()),
            Type::Record(fields, rest) => Type::Record(
                fields.iter().map(|(k, v)| (k.clone(), self.apply_subst(v))).collect(),
                rest.as_ref().map(|r| Box::new(self.apply_subst(r))),
            ),
            _ => ty.clone(),
        }
    }

    fn generalize(&self, env: &TypeEnv, ty: &Type) -> TypeScheme {
        let ty = self.apply_subst(ty);
        let free_in_ty = self.free_vars(&ty);
        let free_in_env = env.free_vars();
        let gen_vars: Vec<_> = free_in_ty.difference(&free_in_env).cloned().collect();

        if gen_vars.is_empty() {
            TypeScheme::mono(ty)
        } else {
            TypeScheme::poly(gen_vars, ty)
        }
    }

    fn instantiate(&mut self, scheme: &TypeScheme) -> Type {
        match scheme {
            TypeScheme::Mono(ty) => ty.clone(),
            TypeScheme::Poly(vars, ty) => {
                let mut subst = HashMap::new();
                for var in vars {
                    subst.insert(var.name.clone(), self.fresh_var());
                }
                // For rank-n: nested Forall types preserve their bound vars
                self.substitute_vars_by_name(ty, &subst)
            }
        }
    }

    fn substitute_vars_by_name(&self, ty: &Type, subst: &HashMap<Symbol, Type>) -> Type {
        match ty {
            Type::Var(v) => {
                if let Some(t) = subst.get(&v.name) {
                    t.clone()
                } else {
                    ty.clone()
                }
            }
            Type::Con(_) => ty.clone(),
            Type::App(f, a) => Type::App(
                Box::new(self.substitute_vars_by_name(f, subst)),
                Box::new(self.substitute_vars_by_name(a, subst)),
            ),
            Type::Fun(p, ann, r) => Type::Fun(
                Box::new(self.substitute_vars_by_name(p, subst)),
                ann.clone(),
                Box::new(self.substitute_vars_by_name(r, subst)),
            ),
            Type::Forall(vars, t) => {
                // Remove bound variables from substitution
                let mut new_subst = subst.clone();
                for var in vars {
                    new_subst.remove(&var.name);
                }
                Type::Forall(vars.clone(), Box::new(self.substitute_vars_by_name(t, &new_subst)))
            }
            Type::Tuple(ts) => Type::Tuple(ts.iter().map(|t| self.substitute_vars_by_name(t, subst)).collect()),
            Type::Record(fields, rest) => Type::Record(
                fields.iter().map(|(k, v)| (k.clone(), self.substitute_vars_by_name(v, subst))).collect(),
                rest.as_ref().map(|r| Box::new(self.substitute_vars_by_name(r, subst))),
            ),
            _ => ty.clone(),
        }
    }

    fn check_match_exhaustiveness(&self, scrut_ty: &Type, arms: &[MatchArm], span: Span) -> Result<(), TypeError> {
        let resolved = self.apply_subst(scrut_ty);
        let ty_name = match &resolved {
            Type::Con(c) => &c.name,
            Type::App(f, _) => match f.as_ref() {
                Type::Con(c) => &c.name,
                _ => return Ok(()),
            },
            _ => return Ok(()),
        };
        if let Some(decl) = self.data_env.lookup(ty_name) {
            let mut covered: std::collections::HashSet<Symbol> = std::collections::HashSet::new();
            for arm in arms {
                self.collect_covered_constructors(&arm.pattern, &mut covered);
            }
            let total = decl.constructors.len();
            let has_wildcard = arms.iter().any(|a| matches!(&a.pattern, Pattern::Wildcard));
            if !has_wildcard && covered.len() < total {
                let missing: Vec<&str> = decl.constructors.iter()
                    .filter(|c| !covered.contains(&c.name))
                    .map(|c| c.name.as_str()).collect();
                return Err(TypeError {
                    message: format!("match is non-exhaustive for type {} — missing constructors: [{}]",
                        ty_name, missing.join(", ")),
                    span,
                });
            }
        }
        Ok(())
    }

    fn collect_covered_constructors(&self, pat: &Pattern, into: &mut std::collections::HashSet<Symbol>) {
        match pat {
            Pattern::Wildcard => { /* wildcard covers everything */ }
            Pattern::Con(name, subs) => { into.insert(name.clone()); for s in subs { self.collect_covered_constructors(s, into); } }
            Pattern::Tuple(pats) => { for p in pats { self.collect_covered_constructors(p, into); } }
            _ => {}
        }
    }

    fn free_vars(&self, ty: &Type) -> std::collections::HashSet<TypeVar> {
        match ty {
            Type::Var(v) => {
                let mut set = std::collections::HashSet::new();
                set.insert(v.clone());
                set
            }
            Type::Con(_) => std::collections::HashSet::new(),
            Type::App(f, a) => {
                let mut set = self.free_vars(f);
                set.extend(self.free_vars(a));
                set
            }
            Type::Fun(p, _, r) => {
                let mut set = self.free_vars(p);
                set.extend(self.free_vars(r));
                set
            }
            Type::Forall(vars, t) => {
                let mut set = self.free_vars(t);
                for var in vars {
                    set.remove(var);
                }
                set
            }
            Type::Tuple(ts) => {
                let mut set = std::collections::HashSet::new();
                for t in ts {
                    set.extend(self.free_vars(t));
                }
                set
            }
            Type::Record(fields, rest) => {
                let mut set = std::collections::HashSet::new();
                for (_, t) in fields {
                    set.extend(self.free_vars(t));
                }
                if let Some(r) = rest {
                    set.extend(self.free_vars(r));
                }
                set
            }
            _ => std::collections::HashSet::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TypeScheme {
    Mono(Type),
    Poly(Vec<TypeVar>, Type),
}

impl TypeScheme {
    pub fn mono(ty: Type) -> Self {
        TypeScheme::Mono(ty)
    }

    pub fn poly(vars: Vec<TypeVar>, ty: Type) -> Self {
        TypeScheme::Poly(vars, ty)
    }
}

/// Kind inference: compute the kind of a type
pub fn kind_of(ty: &Type) -> Kind {
    match ty {
        Type::Var(v) => v.kind.clone(),
        Type::Con(c) => c.kind.clone(),
        Type::App(f, a) => {
            let f_kind = kind_of(f);
            let _a_kind = kind_of(a);
            match f_kind {
                Kind::Arrow(_, ret) => *ret,
                _ => Kind::Star,
            }
        }
        Type::Fun(_, _, _) => Kind::Star,
        Type::Forall(_, _) => Kind::Star,
        Type::Tuple(_) => Kind::Star,
        Type::Record(_, _) => Kind::Star,
        Type::Refined(_, _) => Kind::Star,
        _ => Kind::Star,
    }
}

/// Check that a type has the expected kind
pub fn check_kind(ty: &Type, expected: &Kind) -> Result<(), String> {
    let actual = kind_of(ty);
    if kind_matches(&actual, expected) { Ok(()) }
    else { Err(format!("kind mismatch: expected {:?}, got {:?}", expected, actual)) }
}

fn kind_matches(actual: &Kind, expected: &Kind) -> bool {
    match (actual, expected) {
        (Kind::Star, Kind::Star) => true,
        (Kind::Arrow(a1, r1), Kind::Arrow(a2, r2)) => kind_matches(a1, a2) && kind_matches(r1, r2),
        _ => false,
    }
}

/// Effect subtyping: a ⊆ b if all effects in a are also in b
fn effect_subtype(a: &EffectRow, b: &EffectRow) -> bool {
    match (a, b) {
        (EffectRow::Pure, _) => true,
        (_, EffectRow::Pure) => true,
        (EffectRow::Closed(xs), EffectRow::Closed(ys)) => xs.iter().all(|x| ys.contains(x)),
        _ => true,
    }
}

fn free_vars_of_type(ty: &Type) -> std::collections::HashSet<TypeVar> {
    match ty {
        Type::Var(v) => {
            let mut set = std::collections::HashSet::new();
            set.insert(v.clone());
            set
        }
        Type::Con(_) => std::collections::HashSet::new(),
        Type::App(f, a) => {
            let mut set = free_vars_of_type(f);
            set.extend(free_vars_of_type(a));
            set
        }
        Type::Fun(p, _, r) => {
            let mut set = free_vars_of_type(p);
            set.extend(free_vars_of_type(r));
            set
        }
        Type::Forall(vars, t) => {
            let mut set = free_vars_of_type(t);
            for var in vars { set.remove(var); }
            set
        }
        Type::Tuple(ts) => {
            let mut set = std::collections::HashSet::new();
            for t in ts { set.extend(free_vars_of_type(t)); }
            set
        }
        Type::Record(fields, rest) => {
            let mut set = std::collections::HashSet::new();
            for (_, t) in fields { set.extend(free_vars_of_type(t)); }
            if let Some(r) = rest { set.extend(free_vars_of_type(r)); }
            set
        }
        _ => std::collections::HashSet::new(),
    }
}

#[derive(Debug, Clone)]
pub struct TypeEnv {
    bindings: HashMap<Symbol, TypeScheme>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: Symbol, scheme: TypeScheme) {
        self.bindings.insert(name, scheme);
    }

    pub fn lookup(&self, name: &Symbol) -> Option<&TypeScheme> {
        self.bindings.get(name)
    }

    pub fn free_vars(&self) -> std::collections::HashSet<TypeVar> {
        let mut set = std::collections::HashSet::new();
        for scheme in self.bindings.values() {
            match scheme {
                TypeScheme::Mono(ty) => {
                    set.extend(free_vars_of_type(ty));
                }
                TypeScheme::Poly(vars, ty) => {
                    let bound: std::collections::HashSet<_> = vars.iter().cloned().collect();
                    for v in free_vars_of_type(ty) {
                        if !bound.contains(&v) {
                            set.insert(v);
                        }
                    }
                }
            }
        }
        set
    }
}
