//! 泛型编译期特化(§22.4):对 GenericDef 的构造器类型调用,
//! 匹配对应 defmethod 的 Con(构造器类型)模式,生成特化 def 并替换调用点。
//! 未特化调用保持运行时分发(现有 generic_table 语义)。

use std::collections::HashMap;
use tisp_core::core_ast::{CoreDef, CoreExpr, CoreExprNode, CoreProgram, Lambda, Literal, Param, Pattern, Visibility};
use tisp_core::symbol::Symbol;

pub struct Specializer {
    /// 特化数量(统计输出)
    pub specialized: usize,
    /// 构造器名 → 类型名(用于把调用点构造器映射到方法模式类型)
    ctor_types: HashMap<String, String>,
}

impl Specializer {
    pub fn new() -> Self {
        Self { specialized: 0, ctor_types: HashMap::new() }
    }

    /// 对程序执行泛型特化,返回特化后的程序
    pub fn specialize(&mut self, program: &CoreProgram) -> CoreProgram {
        self.specialized = 0;
        // 构造器 → 类型映射(§22.4 类型驱动特化)
        self.ctor_types.clear();
        for decl in &program.data_decls {
            for ctor in &decl.constructors {
                self.ctor_types.insert(ctor.name.as_str().to_string(), decl.name.as_str().to_string());
            }
        }
        // 1) 收集方法表:generic → [(模式列表, body)]
        let mut methods: HashMap<String, Vec<(Vec<Pattern>, CoreExpr)>> = HashMap::new();
        for def in &program.defs {
            if let CoreExprNode::MethodDef(gen, _cat, patterns, body) = &def.body.node {
                methods.entry(gen.as_str().to_string()).or_default()
                    .push((patterns.clone(), (**body).clone()));
            }
        }
        if methods.is_empty() {
            return program.clone();
        }

        // 2) 遍历 defs,特化构造器类型调用
        let mut spec_defs: Vec<CoreDef> = Vec::new();
        let mut new_defs = Vec::new();
        let mut next_id = 0usize;
        for def in &program.defs {
            let (body, specs) = self.specialize_body(&def.body, &methods, &mut next_id);
            spec_defs.extend(specs);
            new_defs.push(CoreDef {
                name: def.name.clone(),
                ty: def.ty.clone(),
                effects: def.effects.clone(),
                grade: def.grade.clone(),
                mode: def.mode.clone(),
                mode_sigs: def.mode_sigs.clone(),
                determinism: def.determinism.clone(),
                region: def.region.clone(),
                visibility: def.visibility.clone(),
                body,
                requires: def.requires.clone(),
                ensures: def.ensures.clone(),
                span: def.span.clone(),
            });
        }
        new_defs.extend(spec_defs);
        CoreProgram {
            data_decls: program.data_decls.clone(),
            effect_decls: program.effect_decls.clone(),
            type_families: program.type_families.clone(),
            resource_algebras: program.resource_algebras.clone(),
            defs: new_defs,
            pragmas: vec![],
        }
    }

    /// 递归重建表达式:泛型构造器类型调用 → 特化 def 调用
    fn specialize_body(
        &mut self,
        expr: &CoreExpr,
        methods: &HashMap<String, Vec<(Vec<Pattern>, CoreExpr)>>,
        next_id: &mut usize,
    ) -> (CoreExpr, Vec<CoreDef>) {
        let span = expr.span.clone();
        let mut specs = Vec::new();
        let node = match &expr.node {
            CoreExprNode::App(f, arg) => {
                // 柯里化链:收集 (callee, args)
                let mut chain: Vec<&CoreExpr> = vec![arg];
                let mut cur = f;
                while let CoreExprNode::App(inner_f, inner_a) = &cur.node {
                    chain.push(inner_a);
                    cur = inner_f;
                }
                if let CoreExprNode::Var(name) = &cur.node {
                    if let Some(ms) = methods.get(name.as_str()) {
                        // §22.4 类型驱动:多参数构造器类型匹配
                        if let Some((patterns, body)) = ms.iter().find(|(pats, _)| {
                            pats.len() == chain.len()
                                && pats.iter().zip(&chain).all(|(pat, arg)| self.pat_matches(pat, arg))
                        }) {
                            let bound_vars: Vec<Symbol> = patterns.iter().flat_map(pattern_vars).collect();
                            *next_id += 1;
                            let spec_name = Symbol::new(&format!("{}__spec_{}", name, next_id));
                            let spec_body = if bound_vars.is_empty() {
                                body.clone()
                            } else {
                                let params: Vec<Param> = bound_vars.iter().map(|v| Param {
                                    name: v.clone(),
                                    ty: None,
                                    grade: tisp_core::types::Grade::Omega,
                                    mode: tisp_core::types::Mode::In,
                                }).collect();
                                CoreExpr::new(CoreExprNode::Lam(Lambda {
                                    params,
                                    body: Box::new(body.clone()),
                                    ret_type: None,
                                }), span.clone())
                            };
                            let spec_def = CoreDef {
                                name: spec_name.clone(),
                                ty: None,
                                effects: tisp_core::types::EffectRow::Pure,
                                grade: tisp_core::types::Grade::Omega,
                                mode: tisp_core::types::Mode::In,
                                mode_sigs: vec![],
                                determinism: tisp_core::types::Determinism::Det,
                                region: None,
                                visibility: Visibility::Public,
                                body: spec_body,
                                requires: None,
                                ensures: None,
                                span: span.clone(),
                            };
                            specs.push(spec_def);
                            self.specialized += 1;
                            // 调用替换为 (spec_name a1 ... an)
                            let mut call = CoreExpr::new(CoreExprNode::Var(spec_name), span.clone());
                            for a in &chain {
                                call = CoreExpr::new(
                                    CoreExprNode::App(Box::new(call), Box::new((**a).clone())),
                                    span.clone(),
                                );
                            }
                            return (call, specs);
                        }
                    }
                }
                // 未特化:递归重建两侧
                let (f2, mut s1) = self.specialize_body(f, methods, next_id);
                let (a2, s2) = self.specialize_body(arg, methods, next_id);
                specs.append(&mut s1);
                specs.extend(s2);
                CoreExprNode::App(Box::new(f2), Box::new(a2))
            }
            CoreExprNode::If(c, t, e) => {
                let (c2, s1) = self.specialize_body(c, methods, next_id);
                let (t2, s2) = self.specialize_body(t, methods, next_id);
                let (e2, s3) = self.specialize_body(e, methods, next_id);
                specs.extend(s1); specs.extend(s2); specs.extend(s3);
                CoreExprNode::If(Box::new(c2), Box::new(t2), Box::new(e2))
            }
            CoreExprNode::Let(n, ty, v, body) => {
                let (v2, s1) = self.specialize_body(v, methods, next_id);
                let (b2, s2) = self.specialize_body(body, methods, next_id);
                specs.extend(s1); specs.extend(s2);
                CoreExprNode::Let(n.clone(), ty.clone(), Box::new(v2), Box::new(b2))
            }
            CoreExprNode::Do(items) => {
                let mut new_items = Vec::new();
                for i in items {
                    let (i2, s) = self.specialize_body(i, methods, next_id);
                    specs.extend(s);
                    new_items.push(i2);
                }
                CoreExprNode::Do(new_items)
            }
            CoreExprNode::Lam(lam) => {
                let (b2, s) = self.specialize_body(&lam.body, methods, next_id);
                specs.extend(s);
                CoreExprNode::Lam(Lambda {
                    params: lam.params.clone(),
                    body: Box::new(b2),
                    ret_type: lam.ret_type.clone(),
                })
            }
            other => other.clone(),
        };
        (CoreExpr::new(node, span), specs)
    }

    /// 方法模式是否匹配调用实参:Lit 匹配字面量;Con(type) 要求实参为构造器应用且构造器类型名匹配
    fn pat_matches(&self, pat: &Pattern, arg: &CoreExpr) -> bool {
        match (pat, &arg.node) {
            (Pattern::Lit(Literal::I64(a)), CoreExprNode::Lit(Literal::I64(b))) => a == b,
            (Pattern::Lit(Literal::I32(a)), CoreExprNode::Lit(Literal::I32(b))) => a == b,
            (Pattern::Lit(Literal::Bool(a)), CoreExprNode::Lit(Literal::Bool(b))) => a == b,
            (Pattern::Lit(Literal::String(a)), CoreExprNode::Lit(Literal::String(b))) => a == b,
            (Pattern::Con(type_name, _), _) => {
                if let Some(ctor) = app_head(arg) {
                    self.ctor_types.get(ctor.as_str()).map_or(false, |t| t == type_name.as_str())
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

/// 展开应用链取最内层 Var(构造器/函数名)
fn app_head(expr: &CoreExpr) -> Option<&Symbol> {
    match &expr.node {
        CoreExprNode::Var(name) => Some(name),
        CoreExprNode::App(f, _) => app_head(f),
        _ => None,
    }
}

/// 提取模式中绑定的变量名(特化 def 的参数)
fn pattern_vars(pat: &Pattern) -> Vec<Symbol> {
    match pat {
        Pattern::Var(n) => vec![n.clone()],
        Pattern::Con(_, subs) => subs.iter().flat_map(pattern_vars).collect(),
        Pattern::Tuple(subs) => subs.iter().flat_map(pattern_vars).collect(),
        Pattern::Or(subs) => subs.iter().flat_map(pattern_vars).collect(),
        _ => vec![],
    }
}
