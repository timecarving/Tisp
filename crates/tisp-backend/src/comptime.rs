//! §5/§6/§7 comptime 编译期 pass:求值 Comptime 节点、维护编译期 MOP 知识库、
//! 把 defaspect 的 AdviceDef 在编译期编织成 OOP MethodDef 方法链。

use std::sync::{Arc, Mutex};

use tisp_core::core_ast::*;
use tisp_core::evolp::Program;
use tisp_core::span::Span;
use tisp_core::symbol::Symbol;
use tisp_core::types::{Determinism, EffectRow, Grade, Mode};

use crate::interpreter::{EvalError, Interpreter, Value};

pub struct ComptimePass {
    /// 编译期 MOP 知识库(与运行时 KB 分离)
    pub kb: Arc<Mutex<Program>>,
}

impl ComptimePass {
    pub fn new() -> Self {
        Self { kb: Arc::new(Mutex::new(Program::new())) }
    }

    /// 执行 comptime 求值与切面编织,返回变换后的程序
    pub fn run(&self, program: &CoreProgram) -> Result<CoreProgram, String> {
        let mut out = program.clone();
        for def in &mut out.defs {
            let (body, changed) = self.transform_expr(&def.body, program)?;
            let _ = changed;
            def.body = body;
        }

        // AdviceDef → MethodDef(编译期编织进泛型方法表)
        let mut woven: Vec<CoreDef> = Vec::new();
        let mut next_id = 0usize;
        for def in &mut out.defs {
            if let CoreExprNode::AdviceDef(gen, category, patterns, advice) = &def.body.node {
                let gen = gen.clone();
                let category = category.clone();
                let patterns = patterns.clone();
                let advice = (**advice).clone();
                next_id += 1;
                let woven_def = CoreDef {
                    name: Symbol::new(&format!("__woven_{}_{}", gen, next_id)),
                    ty: None,
                    effects: def.effects.clone(),
                    grade: def.grade.clone(),
                    mode: def.mode.clone(),
                    mode_sigs: def.mode_sigs.clone(),
                    determinism: def.determinism.clone(),
                    region: def.region.clone(),
                    visibility: def.visibility.clone(),
                    body: CoreExpr::new(CoreExprNode::MethodDef(gen, category, patterns, Box::new(advice)), def.span),
                    requires: def.requires.clone(),
                    ensures: def.ensures.clone(),
                    span: def.span,
                };
                woven.push(woven_def);
            }
        }
        out.defs.extend(woven);
        Ok(out)
    }

    /// 递归重建表达式:遇到 Comptime(child) 时先递归求值 child,再经解释器
    /// 编译期求值并内联为 Core 表达式。
    fn transform_expr(&self, expr: &CoreExpr, program: &CoreProgram) -> Result<(CoreExpr, bool), String> {
        let span = expr.span.clone();
        let node = match &expr.node {
            CoreExprNode::Comptime(inner) => {
                let (inner, _) = self.transform_expr(inner, program)?;
                let value = self.eval_expr(program, &inner)?;
                return Ok((value_to_core(&value).map_err(|e| format!("comptime 内联失败: {}", e))?, true));
            }
            CoreExprNode::App(f, a) => {
                let (f2, _c1) = self.transform_expr(f, program)?;
                let (a2, _c2) = self.transform_expr(a, program)?;
                CoreExprNode::App(Box::new(f2), Box::new(a2))
            }
            CoreExprNode::Lam(l) => {
                let (b, _c) = self.transform_expr(&l.body, program)?;
                CoreExprNode::Lam(Lambda { params: l.params.clone(), body: Box::new(b), ret_type: l.ret_type.clone() })
            }
            CoreExprNode::Let(n, t, v, b) => {
                let (v2, _c1) = self.transform_expr(v, program)?;
                let (b2, _c2) = self.transform_expr(b, program)?;
                CoreExprNode::Let(n.clone(), t.clone(), Box::new(v2), Box::new(b2))
            }
            CoreExprNode::If(c, t, e) => {
                let (c2, _) = self.transform_expr(c, program)?;
                let (t2, _) = self.transform_expr(t, program)?;
                let (e2, _) = self.transform_expr(e, program)?;
                CoreExprNode::If(Box::new(c2), Box::new(t2), Box::new(e2))
            }
            CoreExprNode::Do(items) => {
                let mut out = Vec::new();
                for i in items { let (x, _) = self.transform_expr(i, program)?; out.push(x); }
                CoreExprNode::Do(out)
            }
            CoreExprNode::Match(s, arms) => {
                let (s2, _) = self.transform_expr(s, program)?;
                let mut new_arms = Vec::new();
                for a in arms {
                    let (b, _) = self.transform_expr(&a.body, program)?;
                    new_arms.push(MatchArm { pattern: a.pattern.clone(), guard: a.guard.clone(), body: Box::new(b) });
                }
                CoreExprNode::Match(Box::new(s2), new_arms)
            }
            CoreExprNode::Handle(e, h) => {
                let (e2, _) = self.transform_expr(e, program)?;
                CoreExprNode::Handle(Box::new(e2), h.clone())
            }
            CoreExprNode::Ann(t, e) => {
                let (e2, _) = self.transform_expr(e, program)?;
                CoreExprNode::Ann(t.clone(), Box::new(e2))
            }
            CoreExprNode::Search(e) => {
                let (e2, _) = self.transform_expr(e, program)?;
                CoreExprNode::Search(Box::new(e2))
            }
            other => other.clone(),
        };
        Ok((CoreExpr::new(node, span), false))
    }

    /// 在独立编译期解释器中求值表达式;get-kb/set-kb 覆盖到编译期 KB
    fn eval_expr(&self, program: &CoreProgram, expr: &CoreExpr) -> Result<Value, String> {
        let mut synthetic = program.clone();
        synthetic.defs.push(CoreDef {
            name: Symbol::new("__comptime"),
            ty: None,
            effects: EffectRow::Pure,
            grade: Grade::Omega,
            mode: Mode::In,
            mode_sigs: vec![],
            determinism: Determinism::Det,
            region: None,
            visibility: Visibility::Public,
            body: CoreExpr::new(CoreExprNode::Lam(Lambda {
                params: vec![],
                body: Box::new(expr.clone()),
                ret_type: None,
            }), Span::dummy()),
            requires: None,
            ensures: None,
            span: Span::dummy(),
        });

        let mut interp = Interpreter::new();
        interp.register_program(&synthetic).map_err(|e| format!("comptime 注册失败: {}", e))?;

        // 编译期 KB 覆盖(与运行时 KB 分离)
        let kb = self.kb.clone();
        let kb_get = kb.clone();
        interp.define(Symbol::new("get-kb"), Value::Builtin("comptime-get-kb".into(), Arc::new(move |_s, _args| {
            let kb = kb_get.lock().unwrap();
            let atoms: im::Vector<Value> = kb.iter().filter_map(|r| match &r.head {
                tisp_core::evolp::LTerm::Fun(n, _) => n.as_str().parse::<i64>().ok().map(Value::Int),
                _ => None,
            }).collect();
            Ok(Value::Vector(atoms))
        })));
        let kb_set = kb.clone();
        interp.define(Symbol::new("set-kb"), Value::Builtin("comptime-set-kb".into(), Arc::new(move |_s, args| {
            use tisp_core::evolp::{LTerm, Rule};
            let mut kb = kb_set.lock().unwrap();
            *kb = Program::new();
            if let Some(v) = args.first() {
                let items = match v {
                    Value::Vector(vs) => vs.iter().cloned().collect::<Vec<_>>(),
                    Value::Data(c, fs) if c.as_str() == "Cons" => crate::interpreter::list_to_vec(v),
                    Value::Data(c, fs) if c.as_str() == "Vec" => fs.clone(),
                    _ => return Err(EvalError { message: "comptime set-kb 需列表".into() }),
                };
                for item in items {
                    if let Value::Int(n) = item {
                        kb.add(Rule::fact(&n.to_string(), LTerm::atom(&n.to_string())));
                    }
                }
            }
            Ok(Value::Unit)
        })));

        interp.eval_expr(expr).map_err(|e| format!("comptime 求值失败: {}", e))
    }
}

/// 把编译期求值结果内联回 Core 表达式(标量/Data/Vector)
fn value_to_core(v: &Value) -> Result<CoreExpr, String> {
    use tisp_core::core_ast::Literal;
    let node = match v {
        Value::Int(n) => CoreExprNode::Lit(Literal::I64(*n)),
        Value::Float(f) => CoreExprNode::Lit(Literal::F64(*f)),
        Value::Bool(b) => CoreExprNode::Lit(Literal::Bool(*b)),
        Value::Str(s) => CoreExprNode::Lit(Literal::String(s.clone())),
        Value::Unit => CoreExprNode::Lit(Literal::Unit),
        Value::Data(name, fields) => {
            let args = fields.iter().map(value_to_core).collect::<Result<Vec<_>, _>>()?;
            CoreExprNode::Data(name.clone(), args)
        }
        Value::Vector(items) => {
            let args = items.iter().map(value_to_core).collect::<Result<Vec<_>, _>>()?;
            CoreExprNode::Data(Symbol::new("Vec"), args)
        }
        other => return Err(format!("comptime 值无法内联: {:?}", other)),
    };
    Ok(CoreExpr::new(node, Span::dummy()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tisp_frontend::{desugar::Desugarer, reader::read};

    fn program(src: &str) -> CoreProgram {
        let forms = read(src).unwrap();
        Desugarer::new().desugar_program(forms).unwrap()
    }

    fn as_int(v: &Value) -> i64 {
        match v { Value::Int(n) => *n, other => panic!("expected int, got {:?}", other) }
    }

    #[test]
    fn test_comptime_folds_literal() {
        let pass = ComptimePass::new();
        let p = program("(defn main [] (+ (comptime (+ 1 2)) 4))");
        let out = pass.run(&p).unwrap();
        let mut interp = Interpreter::new();
        let result = interp.run_program(&out).unwrap().unwrap();
        assert_eq!(as_int(&result), 7, "comptime 应折叠为 3 后参与运算");
    }

    #[test]
    fn test_comptime_error_is_explicit() {
        let pass = ComptimePass::new();
        let p = program("(defn main [] (comptime (no-such-fn 1)))");
        let err = pass.run(&p).unwrap_err();
        assert!(err.contains("comptime"), "编译期错误应带 comptime 上下文,实际: {}", err);
    }

    #[test]
    fn test_compile_time_kb_is_isolated() {
        let pass = ComptimePass::new();
        let p = program("(defn main [] (do (comptime (set-kb [7])) (get-kb)))");
        let out = pass.run(&p).unwrap();
        assert!(!pass.kb.lock().unwrap().is_empty(), "comptime set-kb 应写入编译期 KB");
        let mut interp = Interpreter::new();
        let result = interp.run_program(&out).unwrap().unwrap();
        match result {
            Value::Vector(v) => assert!(v.is_empty(), "运行时 KB 不应包含编译期写入,实际 {:?}", v),
            other => panic!("get-kb 应返回向量,实际 {:?}", other),
        }
    }

    #[test]
    fn test_defaspect_woven_into_method_chain() {
        let pass = ComptimePass::new();
        let p = program("(defgeneric area [x])\n(defmethod area [5] 50)\n(defaspect double-area (pointcut area [x]) :around (* 2 (call-next-method)))\n(defn main [] (area 5))");
        let out = pass.run(&p).unwrap();
        let mut interp = Interpreter::new();
        let result = interp.run_program(&out).unwrap().unwrap();
        assert_eq!(as_int(&result), 100, "around 切面应在编译期编织进方法链");
        assert!(out.defs.iter().any(|d| matches!(&d.body.node, CoreExprNode::MethodDef(gen, MethodCategory::Around, _, _) if gen.as_str() == "area")), "编织后应含 area 的 Around MethodDef");
    }
}
