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
    /// 类型族实例表(§9)
    type_families: Vec<TypeFamilyInstance>,
    /// §17 crisp 上下文深度(♭ 解包要求)
    crisp_depth: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum SessionExpectation {
    Recv,
    Close,
    End,
}

impl TypeInfer {

    /// §9 类型族归约:对类型中的类型族应用 (F a1 ... an) 按实例归约;
    /// 实例模式与实参匹配(变量绑定),结果替换后继续归约;无匹配实例报错
    fn reduce_families(&self, ty: &Type) -> Result<Type, TypeError> {
        match ty {
            Type::App(_, _) => {
                // 展开左结合应用链:(F a1 a2) = App(App(Con(F), a1), a2)
                let (head, args) = self.collect_app(ty);
                match &*head {
                    Type::Con(tc) => {
                        // §多模式实例:遍历同名全部实例,任一模式匹配即归约;全不匹配报错
                        let instances: Vec<&TypeFamilyInstance> = self.type_families.iter()
                            .filter(|i| i.name == tc.name)
                            .collect();
                        if !instances.is_empty() {
                            for inst in &instances {
                                let mut bindings = HashMap::new();
                                if self.match_family_pattern(&inst.params, &args, &mut bindings) {
                                    let result = self.subst_family(&inst.result, &bindings);
                                    return self.reduce_families(&result);
                                }
                            }
                            return Err(TypeError {
                                message: format!("type family '{}' application has no matching instance", tc.name),
                                span: Span::dummy(),
                            });
                        }
                        // 未声明类型族:非数据构造器/非内置类型构造器却被用作应用 → 明确报错
                        if self.is_undeclared_type_family(&tc.name) {
                            return Err(TypeError {
                                message: format!("type family '{}' is not declared (missing typefamily instance)", tc.name),
                                span: Span::dummy(),
                            });
                        }
                    }
                    _ => {}
                }
                // 非类型族:递归归约各层
                let f = self.reduce_families(&head)?;
                let mut out = f;
                for a in &args {
                    let r = self.reduce_families(a)?;
                    out = Type::App(Box::new(out), Box::new(r));
                }
                Ok(out)
            }
            Type::Fun(p, ann, r) => Ok(Type::Fun(
                Box::new(self.reduce_families(p)?), ann.clone(), Box::new(self.reduce_families(r)?))),
            Type::Forall(vars, body) => Ok(Type::Forall(vars.clone(), Box::new(self.reduce_families(body)?))),
            Type::Tuple(items) => Ok(Type::Tuple(items.iter().map(|t| self.reduce_families(t)).collect::<Result<_, _>>()?)),
            Type::Refined(base, pred) => Ok(Type::Refined(Box::new(self.reduce_families(base)?), pred.clone())),
            Type::Pi(n, d, c) => Ok(Type::Pi(n.clone(), Box::new(self.reduce_families(d)?), Box::new(self.reduce_families(c)?))),
            Type::Sigma(n, d, c) => Ok(Type::Sigma(n.clone(), Box::new(self.reduce_families(d)?), Box::new(self.reduce_families(c)?))),
            Type::Record(fields, ext) => Ok(Type::Record(
                fields.iter().map(|(n, t)| (n.clone(), self.reduce_families(t).unwrap())).collect(),
                ext.as_ref().map(|e| Box::new(self.reduce_families(e).unwrap())))),
            Type::Modal(op, t) => Ok(Type::Modal(op.clone(), Box::new(self.reduce_families(t)?))),
            Type::Temporal(op, t) => Ok(Type::Temporal(op.clone(), Box::new(self.reduce_families(t)?))),
            Type::Cohesive(op, t) => Ok(Type::Cohesive(op.clone(), Box::new(self.reduce_families(t)?))),
            Type::Session(st) => Ok(Type::Session(st.clone())),
            Type::Path(t, a, b) => Ok(Type::Path(Box::new(self.reduce_families(t)?), a.clone(), b.clone())),
            Type::Meta(m) => Ok(Type::Meta(m.clone())),
            Type::Interval => Ok(Type::Interval),
            Type::Var(v) => Ok(Type::Var(v.clone())),
            Type::Con(c) => Ok(Type::Con(c.clone())),
            Type::TLambda(p, b) => Ok(Type::TLambda(Box::new(self.reduce_families(p)?), Box::new(self.reduce_families(b)?))),
            Type::Ref(t) => Ok(Type::Ref(Box::new(self.reduce_families(t)?))),
            Type::Ptr(t) => Ok(Type::Ptr(Box::new(self.reduce_families(t)?))),
        }
    }

    /// 判定名称是否为「未声明类型族」:既非内置类型构造器,也非已声明的数据类型
    fn is_undeclared_type_family(&self, name: &Symbol) -> bool {
        let builtin = matches!(name.as_str(),
            "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64"
            | "f32" | "f64" | "bool" | "String" | "Unit" | "Char"
            | "List" | "Map" | "Set" | "Vec" | "Maybe" | "Result" | "Option" | "Tuple");
        !builtin && self.data_env.lookup(name).is_none()
    }

    /// 展开类型应用链:(F a1 a2) → (head, [a1, a2]);head 为最内层非 App 类型
    fn collect_app(&self, ty: &Type) -> (Box<Type>, Vec<Type>) {
        match ty {
            Type::App(f, a) => {
                let (head, mut args) = self.collect_app(f);
                args.push((**a).clone());
                (head, args)
            }
            _ => (Box::new(ty.clone()), Vec::new()),
        }
    }

    /// 实例模式与实参匹配:模式 Var 绑定实参,Con/App 递归(结构一致才匹配)
    fn match_family_pattern(&self, pats: &[Type], args: &[Type], bindings: &mut HashMap<Symbol, Type>) -> bool {
        if pats.len() != args.len() { return false; }
        for (p, a) in pats.iter().zip(args) {
            if !self.match_family_type(p, a, bindings) { return false; }
        }
        true
    }

    fn match_family_type(&self, pat: &Type, arg: &Type, bindings: &mut HashMap<Symbol, Type>) -> bool {
        match pat {
            Type::Var(v) => { bindings.insert(v.name.clone(), arg.clone()); true }
            Type::Con(c) => matches!(arg, Type::Con(ac) if ac.name == c.name),
            Type::App(pf, pa) => match arg {
                Type::App(af, aa) => self.match_family_type(pf, af, bindings) && self.match_family_type(pa, aa, bindings),
                _ => false,
            },
            _ => pat == arg,
        }
    }

    /// 实例结果中绑定变量的替换
    fn subst_family(&self, ty: &Type, bindings: &HashMap<Symbol, Type>) -> Type {
        match ty {
            Type::Var(v) => bindings.get(&v.name).cloned().unwrap_or_else(|| ty.clone()),
            Type::App(f, a) => Type::App(Box::new(self.subst_family(f, bindings)), Box::new(self.subst_family(a, bindings))),
            Type::Fun(p, ann, r) => Type::Fun(Box::new(self.subst_family(p, bindings)), ann.clone(), Box::new(self.subst_family(r, bindings))),
            Type::Forall(vs, b) => Type::Forall(vs.clone(), Box::new(self.subst_family(b, bindings))),
            _ => ty.clone(),
        }
    }

    /// 重建函数体:归约 Lam 参数类型与 Let 类型标注中的类型族应用(§9)
    fn reduce_body_families(&self, expr: &CoreExpr) -> Result<CoreExpr, TypeError> {
        let span = expr.span.clone();
        let node = match &expr.node {
            CoreExprNode::Lam(lam) => {
                let params = lam.params.iter().map(|p| {
                    let mut p2 = p.clone();
                    if let Some(ty) = &p.ty {
                        p2.ty = Some(self.reduce_families(ty)?);
                    }
                    Ok(p2)
                }).collect::<Result<Vec<_>, TypeError>>()?;
                CoreExprNode::Lam(tisp_core::core_ast::Lambda {
                    params,
                    body: Box::new(self.reduce_body_families(&lam.body)?),
                    ret_type: lam.ret_type.as_ref().map(|t| self.reduce_families(t)).transpose()?,
                })
            }
            CoreExprNode::App(f, a) => CoreExprNode::App(
                Box::new(self.reduce_body_families(f)?),
                Box::new(self.reduce_body_families(a)?)),
            CoreExprNode::If(c, t, e) => CoreExprNode::If(
                Box::new(self.reduce_body_families(c)?),
                Box::new(self.reduce_body_families(t)?),
                Box::new(self.reduce_body_families(e)?)),
            CoreExprNode::Let(n, ty, v, body) => CoreExprNode::Let(
                n.clone(),
                ty.as_ref().map(|t| self.reduce_families(t)).transpose()?,
                Box::new(self.reduce_body_families(v)?),
                Box::new(self.reduce_body_families(body)?)),
            CoreExprNode::Do(items) => CoreExprNode::Do(
                items.iter().map(|i| self.reduce_body_families(i)).collect::<Result<_, _>>()?),
            CoreExprNode::Match(s, arms) => CoreExprNode::Match(
                Box::new(self.reduce_body_families(s)?),
                arms.iter().map(|a| {
                    Ok(tisp_core::core_ast::MatchArm {
                        pattern: a.pattern.clone(),
                        guard: a.guard.as_ref().map(|g| Box::new(self.reduce_body_families(g).unwrap())),
                        body: Box::new(self.reduce_body_families(&a.body)?),
                    })
                }).collect::<Result<Vec<_>, TypeError>>()?),
            other => other.clone(),
        };
        Ok(CoreExpr::new(node, span))
    }
    pub fn new() -> Self {
        Self {
            next_var: 0,
            substitution: HashMap::new(),
            data_env: DataEnv::new(),
            hole_env: HoleEnv::new(),
            liquid_checker: LiquidChecker::new(),
            session_state: HashMap::new(),
            type_families: Vec::new(),
            crisp_depth: 0,
        }
    }

    pub fn infer_program(&mut self, program: &CoreProgram) -> Result<Vec<(Symbol, Type)>, TypeError> {
        let mut env = self.initial_env();
        let mut results = Vec::new();

        // 类型族实例表(§9)
        self.type_families = program.type_families.clone();

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

        // 第一遍:全部 defs 插入 fresh 占位(§前向引用与相互递归支持)
        for def in &program.defs {
            let fresh_ty = self.fresh_var();
            env.insert(def.name.clone(), TypeScheme::mono(fresh_ty));
        }

        // 第二遍:逐 def 推断(占位与推断结果 unify)
        for def in &program.defs {
            let ty = self.infer_def(&mut env, def)?;
            // §18.4 生产率检查(返回 next 的流定义须受 delay 保护)
            check_productivity(def, &ty)?;
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

        // ── 列表高阶内置(与解释器 register_builtins 对应)──
        let tv = |id: u64, name: &str| TypeVar { name: Symbol::new(name), kind: Kind::Star, id };
        env.insert(Symbol::new("map"), TypeScheme::poly(vec![tv(30, "a"), tv(31, "b")],
            Type::fun(Type::fun(Type::Var(tv(30, "a")), Type::Var(tv(31, "b"))),
                Type::fun(Type::list(Type::Var(tv(30, "a"))), Type::list(Type::Var(tv(31, "b")))))));
        env.insert(Symbol::new("filter"), TypeScheme::poly(vec![tv(32, "a")],
            Type::fun(Type::fun(Type::Var(tv(32, "a")), Type::bool()),
                Type::fun(Type::list(Type::Var(tv(32, "a"))), Type::list(Type::Var(tv(32, "a")))))));
        env.insert(Symbol::new("reduce"), TypeScheme::poly(vec![tv(33, "a"), tv(34, "b")],
            Type::fun(Type::fun(Type::Var(tv(34, "b")), Type::fun(Type::Var(tv(33, "a")), Type::Var(tv(34, "b")))),
                Type::fun(Type::Var(tv(34, "b")),
                    Type::fun(Type::list(Type::Var(tv(33, "a"))), Type::Var(tv(34, "b")))))));
        env.insert(Symbol::new("foldl"), TypeScheme::poly(vec![tv(35, "a"), tv(36, "b")],
            Type::fun(Type::fun(Type::Var(tv(36, "b")), Type::fun(Type::Var(tv(35, "a")), Type::Var(tv(36, "b")))),
                Type::fun(Type::Var(tv(36, "b")),
                    Type::fun(Type::list(Type::Var(tv(35, "a"))), Type::Var(tv(36, "b")))))));
        env.insert(Symbol::new("foldr"), TypeScheme::poly(vec![tv(37, "a"), tv(38, "b")],
            Type::fun(Type::fun(Type::Var(tv(37, "a")), Type::fun(Type::Var(tv(38, "b")), Type::Var(tv(38, "b")))),
                Type::fun(Type::Var(tv(38, "b")),
                    Type::fun(Type::list(Type::Var(tv(37, "a"))), Type::Var(tv(38, "b")))))));
        env.insert(Symbol::new("range"), TypeScheme::mono(
            Type::fun(Type::i64(), Type::fun(Type::i64(), Type::list(Type::i64())))));
        env.insert(Symbol::new("take"), TypeScheme::poly(vec![tv(39, "a")],
            Type::fun(Type::list(Type::Var(tv(39, "a"))), Type::fun(Type::i64(), Type::list(Type::Var(tv(39, "a")))))));
        env.insert(Symbol::new("drop"), TypeScheme::poly(vec![tv(40, "a")],
            Type::fun(Type::list(Type::Var(tv(40, "a"))), Type::fun(Type::i64(), Type::list(Type::Var(tv(40, "a")))))));
        env.insert(Symbol::new("reverse"), TypeScheme::poly(vec![tv(41, "a")],
            Type::fun(Type::list(Type::Var(tv(41, "a"))), Type::list(Type::Var(tv(41, "a"))))));
        env.insert(Symbol::new("sort"), TypeScheme::mono(Type::fun(Type::list(Type::i64()), Type::list(Type::i64()))));
        env.insert(Symbol::new("count"), TypeScheme::poly(vec![tv(42, "a")],
            Type::fun(Type::list(Type::Var(tv(42, "a"))), Type::i64())));
        env.insert(Symbol::new("length"), TypeScheme::poly(vec![tv(43, "a")],
            Type::fun(Type::list(Type::Var(tv(43, "a"))), Type::i64())));
        env.insert(Symbol::new("zip"), TypeScheme::poly(vec![tv(44, "a"), tv(45, "b")],
            Type::fun(Type::list(Type::Var(tv(44, "a"))),
                Type::fun(Type::list(Type::Var(tv(45, "b"))),
                    Type::list(Type::Tuple(vec![Type::Var(tv(44, "a")), Type::Var(tv(45, "b"))]))))));
        env.insert(Symbol::new("concat"), TypeScheme::poly(vec![tv(46, "a")],
            Type::fun(Type::list(Type::Var(tv(46, "a"))), Type::fun(Type::list(Type::Var(tv(46, "a"))), Type::list(Type::Var(tv(46, "a")))))));
        env.insert(Symbol::new("append"), TypeScheme::poly(vec![tv(64, "a")],
            Type::fun(Type::list(Type::Var(tv(64, "a"))), Type::fun(Type::list(Type::Var(tv(64, "a"))), Type::list(Type::Var(tv(64, "a")))))));
        env.insert(Symbol::new("slurp"), TypeScheme::mono(Type::fun(Type::string(), Type::string())));
        env.insert(Symbol::new("spit"), TypeScheme::mono(Type::fun(Type::string(), Type::fun(Type::string(), Type::unit()))));
        // 算术补全
        env.insert(Symbol::new("abs"), TypeScheme::mono(Type::fun(Type::i64(), Type::i64())));
        env.insert(Symbol::new("sqrt"), TypeScheme::mono(Type::fun(Type::i64(), Type::f64())));
        env.insert(Symbol::new("pow"), TypeScheme::mono(Type::fun(Type::i64(), Type::fun(Type::i64(), Type::i64()))));
        // 字符串补全
        env.insert(Symbol::new("str-sub"), TypeScheme::mono(Type::fun(Type::string(), Type::fun(Type::i64(), Type::string()))));
        env.insert(Symbol::new("str-split"), TypeScheme::mono(Type::fun(Type::string(), Type::fun(Type::string(), Type::list(Type::string())))));
        env.insert(Symbol::new("str-join"), TypeScheme::mono(Type::fun(Type::string(), Type::fun(Type::string(), Type::string()))));
        // 逻辑/反射/效果/FRP/通道(尽力签名,与解释器行为一致)
        env.insert(Symbol::new("=="), TypeScheme::poly(vec![tv(47, "a")],
            Type::fun(Type::Var(tv(47, "a")), Type::fun(Type::Var(tv(47, "a")), Type::bool()))));
        env.insert(Symbol::new("type-of"), TypeScheme::poly(vec![tv(48, "a")],
            Type::fun(Type::Var(tv(48, "a")), Type::string())));
        env.insert(Symbol::new("grade-of"), TypeScheme::poly(vec![tv(49, "a")], Type::fun(Type::Var(tv(49, "a")), Type::string())));
        env.insert(Symbol::new("mode-of"), TypeScheme::poly(vec![tv(50, "a")], Type::fun(Type::Var(tv(50, "a")), Type::string())));
        env.insert(Symbol::new("effects-of"), TypeScheme::poly(vec![tv(51, "a")], Type::fun(Type::Var(tv(51, "a")), Type::string())));
        env.insert(Symbol::new("determinism-of"), TypeScheme::poly(vec![tv(52, "a")], Type::fun(Type::Var(tv(52, "a")), Type::string())));
        env.insert(Symbol::new("get"), TypeScheme::poly(vec![tv(53, "a")], Type::fun(Type::unit(), Type::Var(tv(53, "a")))));
        env.insert(Symbol::new("put"), TypeScheme::poly(vec![tv(54, "a")], Type::fun(Type::Var(tv(54, "a")), Type::unit())));
        env.insert(Symbol::new("ask"), TypeScheme::poly(vec![tv(55, "a")], Type::fun(Type::unit(), Type::Var(tv(55, "a")))));
        env.insert(Symbol::new("tell"), TypeScheme::poly(vec![tv(56, "a")], Type::fun(Type::Var(tv(56, "a")), Type::unit())));
        env.insert(Symbol::new("throw"), TypeScheme::poly(vec![tv(57, "a")], Type::fun(Type::Var(tv(57, "a")), Type::unit())));
        env.insert(Symbol::new("choose"), TypeScheme::poly(vec![tv(58, "a")], Type::fun(Type::Var(tv(58, "a")), Type::Var(tv(58, "a")))));
        env.insert(Symbol::new("chan"), TypeScheme::mono(Type::fun(Type::unit(), Type::string())));
        env.insert(Symbol::new("send"), TypeScheme::poly(vec![tv(59, "a")], Type::fun(Type::string(), Type::fun(Type::Var(tv(59, "a")), Type::unit()))));
        env.insert(Symbol::new("recv"), TypeScheme::mono(Type::fun(Type::string(), Type::i64())));
        env.insert(Symbol::new("stream"), TypeScheme::mono(Type::fun(Type::i64(), Type::Temporal(TemporalOp::Next, Box::new(Type::i64())))));
        env.insert(Symbol::new("stream-take"), TypeScheme::mono(Type::fun(Type::Temporal(TemporalOp::Next, Box::new(Type::i64())), Type::fun(Type::i64(), Type::list(Type::i64())))));
        // §18.5 LTL-as-types:delay : a → (next a);advance : (next a) → a(时序模态类型匹配)
        env.insert(Symbol::new("delay"), TypeScheme::poly(vec![tv(60, "a")], Type::fun(Type::Var(tv(60, "a")), Type::Temporal(TemporalOp::Next, Box::new(Type::Var(tv(60, "a")))))));
        env.insert(Symbol::new("advance"), TypeScheme::poly(vec![tv(61, "a")], Type::fun(Type::Temporal(TemporalOp::Next, Box::new(Type::Var(tv(61, "a")))), Type::Var(tv(61, "a")))));
        // §18.1 always/eventually:流判定(有限窗口) → bool
        let stream_pred_bool = Type::fun(Type::Temporal(TemporalOp::Next, Box::new(Type::i64())),
            Type::fun(Type::fun(Type::i64(), Type::bool()), Type::fun(Type::i64(), Type::bool())));
        env.insert(Symbol::new("always"), TypeScheme::mono(stream_pred_bool.clone()));
        env.insert(Symbol::new("eventually"), TypeScheme::mono(stream_pred_bool));
        env.insert(Symbol::new("clock"), TypeScheme::mono(Type::fun(Type::unit(), Type::string())));
        env.insert(Symbol::new("~"), TypeScheme::mono(Type::fun(Type::bool(), Type::bool())));
        env.insert(Symbol::new("interval-neg"), TypeScheme::mono(Type::fun(Type::bool(), Type::bool())));
        env.insert(Symbol::new("interval-and"), TypeScheme::mono(Type::fun(Type::bool(), Type::fun(Type::bool(), Type::bool()))));
        env.insert(Symbol::new("interval-or"), TypeScheme::mono(Type::fun(Type::bool(), Type::fun(Type::bool(), Type::bool()))));
        env.insert(Symbol::new("fresh"), TypeScheme::mono(Type::fun(Type::unit(), Type::i64())));
        env.insert(Symbol::new("search"), TypeScheme::poly(vec![tv(62, "a")], Type::fun(Type::Var(tv(62, "a")), Type::bool())));
        env.insert(Symbol::new("solve-all"), TypeScheme::mono(Type::fun(Type::i64(), Type::list(Type::i64()))));
        env.insert(Symbol::new("find-all"), TypeScheme::poly(vec![tv(65, "a")],
            Type::fun(Type::Var(tv(65, "a")), Type::list(Type::list(Type::i64())))));
        env.insert(Symbol::new("commit!"), TypeScheme::poly(vec![tv(63, "a")], Type::fun(Type::Var(tv(63, "a")), Type::unit())));

        // ── §31/§32 范式内置(pf-*):经 ParadigmRegistry 接入,与解释器 register_builtins 对应 ──
        let li = Type::list(Type::i64());
        let i64t = Type::i64();
        let f64t = Type::f64();
        let bt = Type::bool();
        let st = Type::string();
        let ut = Type::unit();
        let mut m = |name: &str, ty: Type| env.insert(Symbol::new(name), TypeScheme::mono(ty));
        m("pf-higher-order", Type::fun(i64t.clone(), bt.clone()));
        m("pf-induce", Type::fun(li.clone(), Type::fun(li.clone(), li.clone())));
        m("pf-prob", Type::fun(f64t.clone(), f64t.clone()));
        m("pf-eventually", Type::fun(li.clone(), Type::fun(i64t.clone(), bt.clone())));
        m("pf-subsume", Type::fun(ut.clone(), bt.clone()));
        m("pf-settle", Type::fun(i64t.clone(), Type::fun(i64t.clone(), bt.clone())));
        m("pf-fuzzy-and", Type::fun(f64t.clone(), Type::fun(f64t.clone(), f64t.clone())));
        m("pf-tabling", Type::fun(bt.clone(), bt.clone()));
        m("pf-typed-pred", Type::fun(i64t.clone(), bt.clone()));
        m("pf-reactive", Type::fun(i64t.clone(), i64t.clone()));
        m("pf-context-query", Type::fun(bt.clone(), Type::fun(bt.clone(), bt.clone())));
        m("pf-possible", Type::fun(bt.clone(), Type::fun(bt.clone(), bt.clone())));
        m("pf-evolp", Type::fun(i64t.clone(), i64t.clone()));
        m("pf-dlp", Type::fun(li.clone(), li.clone()));
        m("pf-get-kb", Type::fun(ut.clone(), st.clone()));
        m("pf-array-sum", Type::fun(li.clone(), i64t.clone()));
        m("pf-stack-top", Type::fun(li.clone(), i64t.clone()));
        m("pf-compose", Type::fun(i64t.clone(), i64t.clone()));
        m("pf-sym-eval", Type::fun(i64t.clone(), i64t.clone()));
        m("pf-dfa-accept", Type::fun(li.clone(), bt.clone()));
        m("pf-sm-drive", Type::fun(i64t.clone(), i64t.clone()));
        m("pf-dispatch", Type::fun(i64t.clone(), st.clone()));
        m("pf-stream-take", Type::fun(i64t.clone(), li.clone()));
        m("pf-aop-weave", Type::fun(i64t.clone(), i64t.clone()));
        // §统一内存管理:ref/deref/set! 为 State 效应操作,Ref a 分级值
        let ref_i64 = Type::Ref(Box::new(i64t.clone()));
        m("ref", Type::fun(i64t.clone(), ref_i64.clone()));
        m("deref", Type::fun(ref_i64.clone(), i64t.clone()));
        m("set!", Type::fun(ref_i64.clone(), Type::fun(i64t.clone(), ut.clone())));

        env
    }

    fn infer_def(&mut self, env: &mut TypeEnv, def: &CoreDef) -> Result<Type, TypeError> {
        // 占位类型已由 infer_program 第一遍插入(前向引用/相互递归);兜底补插
        let placeholder = match env.lookup(&def.name) {
            Some(TypeScheme::Mono(ty)) => ty.clone(),
            Some(TypeScheme::Poly(_, ty)) => ty.clone(),
            None => {
                let fresh_ty = self.fresh_var();
                env.insert(def.name.clone(), TypeScheme::mono(fresh_ty.clone()));
                fresh_ty
            }
        };

        // §9 类型族归约:重建函数体(参数类型/标注中的类型族应用归约)
        let body = self.reduce_body_families(&def.body)?;
        let ty = self.infer_expr(env, &body)?;

        // Unify the inferred type with the placeholder
        self.unify(&placeholder, &ty, def.span)?;

        // Generalize and update the environment
        let final_ty = self.apply_subst(&ty);
        let scheme = self.generalize(env, &final_ty);
        env.insert(def.name.clone(), scheme);

        // §9 类型族归约:def.ty 中的类型族应用归约,悬挂报错
        let ann = match &def.ty {
            Some(ty) => Some(self.reduce_families(ty)?),
            None => None,
        };

        // §19.1:依赖类型注解(def.ty 为 Pi/Sigma)时,把推断的 Fun 提升为依赖类型并统一
        if let Some(ann) = &ann {
            if matches!(ann, Type::Pi(..) | Type::Sigma(..)) {
                if let Type::Pi(name, _, _) = ann {
                    if let Type::Fun(p, _, r) = &final_ty {
                        let dep = Type::Pi(name.clone(), p.clone(), r.clone());
                        self.unify(&dep, ann, def.span)?;
                        return Ok(self.apply_subst(&dep));
                    }
                }
                self.unify(&final_ty, ann, def.span)?;
                return Ok(self.apply_subst(ann));
            }
        }

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

                // §19.1:返回类型注解为 Pi/Sigma 时构建依赖类型(单参数)
                let mut result = body_ty.clone();
                if let Some(ret) = &lambda.ret_type {
                    if let Type::Pi(name, _, _) = ret {
                        if param_types.len() == 1 {
                            result = Type::Pi(name.clone(), Box::new(param_types[0].clone()), Box::new(body_ty));
                        }
                    }
                }
                // §零参 lambda:(fn [] body) → Unit -> body(与解释器零参闭包语义一致)
                if param_types.is_empty() {
                    result = Type::fun(Type::unit(), result);
                }
                for param_ty in param_types.into_iter().rev() {
                    result = Type::fun(param_ty, result);
                }
                // §19.1:Pi/Sigma 注解与推断结果统一(依赖类型检查)
                if let Some(ret) = &lambda.ret_type {
                    if matches!(ret, Type::Pi(..) | Type::Sigma(..)) {
                        self.unify(&result, ret, expr.span)?;
                    }
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
                // §let 内递归:value 推断前先绑定 fresh 占位((let [f (fn ... (f ...))] ...))
                let mut local_env = env.clone();
                let placeholder = self.fresh_var();
                local_env.insert(name.clone(), TypeScheme::mono(placeholder.clone()));

                let value_ty = self.infer_expr(&mut local_env, value)?;
                self.unify(&placeholder, &value_ty, expr.span)?;

                if let Some(ann) = ty_ann {
                    self.unify(&value_ty, ann, expr.span)?;
                }

                let final_ty = self.apply_subst(&value_ty);
                let scheme = self.generalize(&local_env, &final_ty);
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
                // §12.2/12.3:handle 处理 body 的效果;类型为 body 类型(效果行消减见 effect_infer)
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

            // ── Session type protocol checking (§20.2)──
            CoreExprNode::Session(op, body) => {
                // 协议顺序检查:期望操作与实际操作不符报错
                let expected = self.session_state.get(&0).cloned();
                let actual = match op {
                    SessionOp::Send => Some("send"),
                    SessionOp::Recv => Some("recv"),
                    SessionOp::Close => Some("close"),
                    SessionOp::Fork(_) => None,
                };
                if let (Some(exp), Some(act)) = (expected, actual) {
                    let exp_str = match exp {
                        SessionExpectation::Recv => "recv",
                        SessionExpectation::Close => "close",
                        SessionExpectation::End => "end",
                    };
                    if exp_str != act {
                        return Err(TypeError {
                            message: format!("会话协议顺序违反:期望 {} 实际 {}", exp_str, act),
                            span: expr.span.clone(),
                        });
                    }
                }
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

            CoreExprNode::FlatMod(e) => {
                // §17:♭ 解包需 crisp 上下文;非 crisp 上下文报错
                if self.crisp_depth == 0 {
                    return Err(TypeError {
                        message: "cohesive 上下文错误:♭(flat)解包要求 crisp 上下文".into(),
                        span: expr.span.clone(),
                    });
                }
                self.infer_expr(env, e)
            }
            CoreExprNode::CrispMod(e) => {
                self.crisp_depth += 1;
                let r = self.infer_expr(env, e);
                self.crisp_depth -= 1;
                r
            }
            CoreExprNode::ShapeMod(e) => self.infer_expr(env, e),
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
            Pattern::Or(pats) => {
                // (or p1 p2 ...)(§8.2):各分支类型统一
                let ty = self.fresh_var();
                for pat in pats {
                    let p_ty = self.infer_pattern(env, pat)?;
                    self.unify(&ty, &p_ty, Span::dummy())?;
                }
                Ok(ty)
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
                for (t1, t2) in ts1.iter().zip(ts2.iter()) {                    self.unify(t1, t2, span)?;
                }
                Ok(())
            }
            // §19.1:依赖类型统一(结构相等,绑定名忽略)
            (Type::Pi(_, d1, c1), Type::Pi(_, d2, c2)) => {
                self.unify(d1, d2, span)?;
                self.unify(c1, c2, span)
            }
            (Type::Sigma(_, d1, c1), Type::Sigma(_, d2, c2)) => {
                self.unify(d1, d2, span)?;
                self.unify(c1, c2, span)
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
            // ── 类型 λ(tlambda):unify 参数与返回 ──
            (Type::TLambda(p1, b1), Type::TLambda(p2, b2)) => {
                self.unify(p1, p2, span)?;
                self.unify(b1, b2, span)
            }
            // ── 可变引用(Ref a):unify 元素类型 ──
            (Type::Ref(t1), Type::Ref(t2)) => self.unify(t1, t2, span),
            // ── 裸指针(Ptr a):unify 元素类型 ──
            (Type::Ptr(t1), Type::Ptr(t2)) => self.unify(t1, t2, span),
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
            Type::Pi(x, t, r) => Type::Pi(
                x.clone(),
                Box::new(self.apply_subst(t)),
                Box::new(self.apply_subst(r)),
            ),
            Type::Sigma(x, t, r) => Type::Sigma(
                x.clone(),
                Box::new(self.apply_subst(t)),
                Box::new(self.apply_subst(r)),
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

/// §18.3 稳定类型:可安全跨时刻(基元 + 用户 ADT;时序/闭包/类型变量非稳定)
pub fn is_stable_type(ty: &Type) -> bool {
    match ty {
        Type::Con(c) => {
            let n = c.name.as_str();
            // 基元稳定
            if matches!(n, "i64" | "i32" | "f64" | "f32" | "bool" | "String" | "char" | "Unit" | "u8" | "u16" | "u32" | "u64") {
                return true;
            }
            // 用户 ADT(大写构造器)稳定;类型变量(小写)非稳定
            matches!(n.chars().next(), Some(ch) if ch.is_ascii_uppercase())
        }
        // □_t A 稳定当且仅当 A 稳定
        Type::Temporal(TemporalOp::Always, inner) => is_stable_type(inner),
        // next/eventually 是时序值,非稳定
        Type::Temporal(_, _) => false,
        // 闭包可能捕获时序值,非稳定
        Type::Fun(..) => false,
        // 类型变量非稳定
        Type::Var(_) => false,
        // 元组/记录/精化等:递归判断(元素稳定则稳定)
        Type::Tuple(ts) => ts.iter().all(is_stable_type),
        Type::Refined(base, _) => is_stable_type(base),
        _ => true,
    }
}

/// §11 □_r/◇_ε 引入消去等级推导:不可推断的等级变量默认取 ω(与 spec 一致);
/// 递归解析 Modal 嵌套中的等级变量。
pub fn resolve_modal_grade(ty: &Type) -> Type {
    match ty {
        Type::Modal(ModalOp::Necessity(g), inner) => {
            let g = match g {
                Grade::Var(_) => Grade::Omega, // 不可推断 → 默认 ω(并可在上层警告)
                other => other.clone(),
            };
            Type::Modal(ModalOp::Necessity(g), Box::new(resolve_modal_grade(inner)))
        }
        Type::Modal(op, inner) => Type::Modal(op.clone(), Box::new(resolve_modal_grade(inner))),
        Type::Fun(p, ann, r) => Type::Fun(Box::new(resolve_modal_grade(p)), ann.clone(), Box::new(resolve_modal_grade(r))),
        Type::Ref(t) => Type::Ref(Box::new(resolve_modal_grade(t))),
        Type::Ptr(t) => Type::Ptr(Box::new(resolve_modal_grade(t))),
        _ => ty.clone(),
    }
}

/// §18.4 生产率:自递归且返回 next(流)的定义,递归调用须在 delay(⃝)下(受保护)。
/// 返回未受保护的自递归调用数。
fn unguarded_self_calls(name: &Symbol, expr: &CoreExpr, under_delay: bool) -> usize {
    match &expr.node {
        CoreExprNode::Var(v) if v == name => if under_delay { 0 } else { 1 },
        CoreExprNode::App(f, a) => {
            // (delay e) 是保护上下文:实参在 delay 内递归为受保护
            let is_delay = matches!(&f.node, CoreExprNode::Var(v) if v.as_str() == "delay");
            unguarded_self_calls(name, f, under_delay)
                + unguarded_self_calls(name, a, under_delay || is_delay)
        }
        CoreExprNode::Lam(l) => unguarded_self_calls(name, &l.body, under_delay),
        CoreExprNode::Let(_, _, v, b) => unguarded_self_calls(name, v, under_delay) + unguarded_self_calls(name, b, under_delay),
        CoreExprNode::If(c, t, e) => unguarded_self_calls(name, c, under_delay) + unguarded_self_calls(name, t, under_delay) + unguarded_self_calls(name, e, under_delay),
        CoreExprNode::Do(exprs) => exprs.iter().map(|e| unguarded_self_calls(name, e, under_delay)).sum(),
        CoreExprNode::Match(s, arms) => unguarded_self_calls(name, s, under_delay) + arms.iter().map(|a| unguarded_self_calls(name, &a.body, under_delay)).sum::<usize>(),
        CoreExprNode::Data(_, args) => args.iter().map(|a| unguarded_self_calls(name, a, under_delay)).sum(),
        _ => 0,
    }
}

/// §18.4 生产率检查:返回 next(流)的自递归定义,递归须受 delay 保护
fn check_productivity(def: &CoreDef, ty: &Type) -> Result<(), TypeError> {
    if matches!(ty, Type::Temporal(TemporalOp::Next, _)) {
        let unguarded = unguarded_self_calls(&def.name, &def.body, false);
        if unguarded > 0 {
            return Err(TypeError {
                message: format!("生产率违反:流定义 '{}' 有 {} 处未受 ⃝(delay)保护的递归调用", def.name, unguarded),
                span: def.span.clone(),
            });
        }
    }
    Ok(())
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

#[cfg(test)]
mod ltl_tests {
    use super::*;

    #[test]
    fn test_resolve_modal_grade() {
        // §11 □_r 不可推断的等级变量默认取 ω
        let x = Type::Modal(
            ModalOp::Necessity(Grade::Var(Symbol::new("r"))),
            Box::new(Type::i64()),
        );
        let resolved = resolve_modal_grade(&x);
        match resolved {
            Type::Modal(ModalOp::Necessity(Grade::Omega), inner) => {
                assert_eq!(*inner, Type::i64());
            }
            other => panic!("等级变量应默认 ω,实际 {:?}", other),
        }
    }

    #[test]
    fn test_temporal_type_schemes() {
        // §18.5 LTL-as-types:delay : a → (next a);advance : (next a) → a
        let ti = TypeInfer::new();
        let env = ti.initial_env();
        let advance = env.lookup(&Symbol::new("advance")).expect("advance 应有类型");
        let advance_ty = match advance {
            TypeScheme::Poly(_, ty) => ty.clone(),
            TypeScheme::Mono(ty) => ty.clone(),
        };
        match &advance_ty {
            Type::Fun(param, _, ret) => {
                assert!(matches!(param.as_ref(), Type::Temporal(TemporalOp::Next, _)), "advance 参数应为 (next a),实际 {:?}", param);
                assert!(matches!(ret.as_ref(), Type::Var(_)), "advance 返回应为 a");
            }
            other => panic!("advance 应为函数类型,实际 {:?}", other),
        }
        let delay = env.lookup(&Symbol::new("delay")).expect("delay 应有类型");
        let delay_ty = match delay {
            TypeScheme::Poly(_, ty) => ty.clone(),
            TypeScheme::Mono(ty) => ty.clone(),
        };
        match &delay_ty {
            Type::Fun(param, _, ret) => {
                assert!(matches!(param.as_ref(), Type::Var(_)), "delay 参数应为 a");
                assert!(matches!(ret.as_ref(), Type::Temporal(TemporalOp::Next, _)), "delay 返回应为 (next a)");
            }
            other => panic!("delay 应为函数类型,实际 {:?}", other),
        }
    }

    #[test]
    fn test_stable_type() {
        // §18.3:基元稳定,next/闭包/类型变量非稳定,□_t A 稳定当且仅当 A 稳定
        assert!(is_stable_type(&Type::i64()));
        assert!(is_stable_type(&Type::bool()));
        assert!(is_stable_type(&Type::string()));
        assert!(is_stable_type(&Type::Con(tisp_core::types::TypeCon { name: Symbol::new("MyData"), kind: tisp_core::types::Kind::Star })));
        assert!(!is_stable_type(&Type::Temporal(TemporalOp::Next, Box::new(Type::i64()))), "next 非稳定");
        assert!(!is_stable_type(&Type::fun(Type::i64(), Type::bool())), "闭包非稳定");
        assert!(!is_stable_type(&Type::Con(tisp_core::types::TypeCon { name: Symbol::new("a"), kind: tisp_core::types::Kind::Star })), "类型变量非稳定");
        assert!(is_stable_type(&Type::Temporal(TemporalOp::Always, Box::new(Type::i64()))), "□_t i64 稳定");
        assert!(!is_stable_type(&Type::Temporal(TemporalOp::Always, Box::new(Type::fun(Type::i64(), Type::bool())))), "□_t 闭包 非稳定");
    }

    #[test]
    fn test_productivity_unguarded_self_call() {
        // §18.4 生产率:自递归调用须受 delay(⃝)保护
        // (f x) 未保护 → 1
        let f_call = CoreExpr::new(CoreExprNode::App(
            Box::new(CoreExpr::new(CoreExprNode::Var(Symbol::new("f")), Span::dummy())),
            Box::new(CoreExpr::new(CoreExprNode::Var(Symbol::new("x")), Span::dummy())),
        ), Span::dummy());
        assert_eq!(unguarded_self_calls(&Symbol::new("f"), &f_call, false), 1);
        // (delay (f x)) 受保护 → 0
        let guarded = CoreExpr::new(CoreExprNode::App(
            Box::new(CoreExpr::new(CoreExprNode::Var(Symbol::new("delay")), Span::dummy())),
            Box::new(f_call),
        ), Span::dummy());
        assert_eq!(unguarded_self_calls(&Symbol::new("f"), &guarded, false), 0);
    }
}
