//! 泛型编译期特化(§22.4):对 GenericDef 的 ground 字面量调用,
//! 匹配对应 defmethod 的 Literal 模式,生成特化 def 并替换调用点。
//! 未特化调用保持运行时分发(现有 generic_table 语义)。

use std::collections::HashMap;
use tisp_core::core_ast::{CoreDef, CoreExpr, CoreExprNode, CoreProgram, Literal, Pattern};
use tisp_core::symbol::Symbol;

pub struct Specializer {
    /// 特化数量(统计输出)
    pub specialized: usize,
}

impl Specializer {
    pub fn new() -> Self {
        Self { specialized: 0 }
    }

    /// 对程序执行泛型特化,返回特化后的程序
    pub fn specialize(&mut self, program: &CoreProgram) -> CoreProgram {
        self.specialized = 0;
        // 1) 收集方法表:generic → [(模式, body)]
        let mut methods: HashMap<String, Vec<(Pattern, CoreExpr)>> = HashMap::new();
        for def in &program.defs {
            if let CoreExprNode::MethodDef(gen, _cat, patterns, body) = &def.body.node {
                if let Some(p) = patterns.first() {
                    methods.entry(gen.as_str().to_string()).or_default()
                        .push((p.clone(), (**body).clone()));
                }
            }
        }
        if methods.is_empty() {
            return program.clone();
        }

        // 2) 遍历 defs,特化字面量调用
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
        }
    }

    /// 递归重建表达式:泛型字面量调用 → 特化 def 调用
    fn specialize_body(
        &mut self,
        expr: &CoreExpr,
        methods: &HashMap<String, Vec<(Pattern, CoreExpr)>>,
        next_id: &mut usize,
    ) -> (CoreExpr, Vec<CoreDef>) {
        let span = expr.span.clone();
        let mut specs = Vec::new();
        let node = match &expr.node {
            CoreExprNode::App(f, arg) => {
                // 柯里化链:收集 (callee, args);callee 是泛型且唯一实参是字面量
                let mut chain: Vec<&CoreExpr> = vec![arg];
                let mut cur = f;
                while let CoreExprNode::App(inner_f, inner_a) = &cur.node {
                    chain.push(inner_a);
                    cur = inner_f;
                }
                if let CoreExprNode::Var(name) = &cur.node {
                    if let Some(ms) = methods.get(name.as_str()) {
                        if chain.len() == 1 {
                            if let CoreExprNode::Lit(lit) = &chain[0].node {
                                // 匹配 Literal 模式
                                if let Some((_, body)) = ms.iter().find(|(p, _)| match (p, lit) {
                                (Pattern::Lit(Literal::I64(a)), Literal::I64(b)) => a == b,
                                (Pattern::Lit(Literal::I32(a)), Literal::I32(b)) => a == b,
                                (Pattern::Lit(Literal::Bool(a)), Literal::Bool(b)) => a == b,
                                (Pattern::Lit(Literal::String(a)), Literal::String(b)) => a == b,
                                _ => false,
                            }) {
                                    *next_id += 1;
                                    let spec_name = Symbol::new(&format!("{}__spec_{}", name, next_id));
                                    let spec_def = CoreDef {
                                        name: spec_name.clone(),
                                        ty: None,
                                        effects: tisp_core::types::EffectRow::Pure,
                                        grade: tisp_core::types::Grade::Omega,
                                        mode: tisp_core::types::Mode::In,
                                        mode_sigs: vec![],
                                        determinism: tisp_core::types::Determinism::Det,
                                        body: body.clone(),
                                        requires: None,
                                        ensures: None,
                                        span: span.clone(),
                                    };
                                    specs.push(spec_def);
                                    self.specialized += 1;
                                    // 调用替换为 (spec_name Unit)(特化 def 无参)
                                    return (CoreExpr::new(
                                        CoreExprNode::App(
                                            Box::new(CoreExpr::new(CoreExprNode::Var(spec_name), span.clone())),
                                            Box::new(CoreExpr::new(CoreExprNode::Lit(tisp_core::core_ast::Literal::Unit), span.clone())),
                                        ),
                                        span,
                                    ), specs);
                                }
                            }
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
                CoreExprNode::Lam(tisp_core::core_ast::Lambda {
                    params: lam.params.clone(),
                    body: Box::new(b2),
                    ret_type: lam.ret_type.clone(),
                })
            }
            other => other.clone(),
        };
        (CoreExpr::new(node, span), specs)
    }
}
