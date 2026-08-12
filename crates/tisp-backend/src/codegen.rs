/// LLVM IR 生成器
///
/// - 启用 `llvm` feature 时:`IrGenerator` 使用 inkwell 生成真实 LLVM IR(SSA 由
///   LLVM 保证合法),支持算术、比较、if/else(phi 汇合)、let、Do、递归函数调用。
/// - 未启用 `llvm` feature 时:回退到 `TextIrGenerator`(可被 llc 编译的文本 IR)。
///
/// 两者共享同一接口 `IrGenerator::generate(&CoreProgram) -> String`,
/// CLI 的 `--ir` 输出在 llvm feature 下为 inkwell 生成的 IR。

use tisp_core::core_ast::*;
use tisp_core::types::Type;
use std::collections::HashMap;

/// Map Tisp type to LLVM IR type string
pub fn tisp_type_to_llvm(ty: &Type) -> String {
    match ty {
        Type::Var(_) => "i64".to_string(),
        Type::Con(c) => match c.name.as_str() {
            "i64" | "i32" | "u64" | "u32" => c.name.as_str().to_string(),
            "i8" | "u8" => "i8".to_string(),
            "i16" | "u16" => "i16".to_string(),
            "f32" => "float".to_string(),
            "f64" => "double".to_string(),
            "bool" => "i1".to_string(),
            "String" => "ptr".to_string(),
            "Unit" => "void".to_string(),
            _ => "ptr".to_string(),
        },
        Type::Fun(..) => "ptr".to_string(),
        Type::App(..) => "ptr".to_string(),
        _ => "i64".to_string(),
    }
}

pub fn tisp_return_type_to_llvm(ty: &Type) -> String {
    match ty {
        Type::Var(_) => "i64".to_string(),
        Type::Con(c) => match c.name.as_str() {
            "Unit" => "void".to_string(),
            "bool" => "i1".to_string(),
            _ => tisp_type_to_llvm(ty),
        },
        _ => "i64".to_string(),
    }
}

/// 统一入口:llvm feature 下用 inkwell 生成真实 IR,否则回退文本生成器
pub struct IrGenerator;

impl IrGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn generate(&mut self, program: &CoreProgram) -> String {
        #[cfg(feature = "llvm")]
        {
            llvm_generate(program)
        }
        #[cfg(not(feature = "llvm"))]
        {
            TextIrGenerator::new().generate(program)
        }
    }
}

/// 把 IR 中最后一条 "ret i64 %X" 转成 "target = add i64 %X, 0"(支持多行)
#[cfg(not(feature = "llvm"))]
fn ret_to_assign(ir: &str, target: &str) -> String {
    if let Some(idx) = ir.rfind("  ret i64 ") {
        let (head, tail) = ir.split_at(idx);
        let val = tail.trim_start().trim_start_matches("ret i64 ").trim();
        format!("{}{} = add i64 {}, 0", head, target, val)
    } else {
        ir.to_string()
    }
}

/// 文本 IR 生成器(无 inkwell 依赖,输出可被 llc 编译)
#[cfg(not(feature = "llvm"))]
struct TextIrGenerator {
    next_label: usize,
    next_reg: usize,
    locals: HashMap<String, String>,
    /// Accumulated IR text from val_to_reg calls
    ir_buf: String,
}

#[cfg(not(feature = "llvm"))]
#[allow(dead_code)]
struct IrFunction;

#[cfg(not(feature = "llvm"))]
impl TextIrGenerator {
    fn new() -> Self {
        Self { next_label: 0, next_reg: 0, locals: HashMap::new(), ir_buf: String::new() }
    }

    fn generate(&mut self, program: &CoreProgram) -> String {
        let mut ir = String::new();

        for def in &program.defs {
            self.locals.clear();
            self.next_reg = 0;
            self.next_label = 0;
            self.ir_buf.clear();

            let body_ir = self.compile_expr(&def.body);
            let fn_ir = format!(
                "define i64 @{}() {{\nentry:\n{}{}\n}}\n",
                def.name.as_str(), self.ir_buf, body_ir
            );
            ir.push_str(&fn_ir);
        }

        ir
    }

    fn compile_expr(&mut self, expr: &CoreExpr) -> String {
        match &expr.node {
            CoreExprNode::Lit(lit) => self.compile_literal(lit),
            CoreExprNode::Var(name) => self.compile_var(name),
            CoreExprNode::App(func, arg) => self.compile_app(func, arg),
            CoreExprNode::Lam(lambda) => self.compile_lambda(lambda),
            CoreExprNode::Let(name, _, value, body) => self.compile_let(name, value, body),
            CoreExprNode::If(cond, then, else_) => self.compile_if(cond, then, else_),
            CoreExprNode::Do(exprs) => {
                let mut last = "  ret i64 0".to_string();
                for e in exprs { last = self.compile_expr(e); }
                last
            }
            _ => "  ret i64 0".to_string(),
        }
    }

    fn compile_literal(&mut self, lit: &Literal) -> String {
        let reg = self.fresh_reg();
        let val = match lit {
            Literal::I64(n) => format!("{}", n),
            Literal::Bool(true) => "1".into(),
            Literal::Bool(false) => "0".into(),
            _ => "0".into(),
        };
        format!("  %{} = add i64 {}, 0\n  ret i64 %{}", reg, val, reg)
    }

    fn compile_var(&mut self, name: &tisp_core::symbol::Symbol) -> String {
        if let Some(reg) = self.locals.get(name.as_str()) {
            format!("  ret i64 {}", reg)
        } else {
            "  ret i64 0".to_string()
        }
    }

    fn compile_app(&mut self, func: &CoreExpr, arg: &CoreExpr) -> String {
        // Handle curried binary operations: (+ 21 21) = App(App(Var("+"), Lit(21)), Lit(21))
        // Outer: func = App(Var("+"), Lit(21)), arg = Lit(21)
        if let CoreExprNode::App(inner_func, inner_arg) = &func.node {
            if let CoreExprNode::Var(op_name) = &inner_func.node {
                let lhs_reg = self.eval_to_reg(inner_arg.as_ref());
                let rhs_reg = self.eval_to_reg(arg);
                let result = self.fresh_reg();
                let op = match op_name.as_str() {
                    "+" => "add", "-" => "sub", "*" => "mul", "/" => "sdiv",
                    _ => "add",
                };
                return format!("  %{} = {} i64 {}, {}\n  ret i64 %{}", result, op, lhs_reg, rhs_reg, result);
            }
        }
        "  ret i64 0".to_string()
    }

    fn compile_lambda(&mut self, lambda: &Lambda) -> String {
        self.compile_expr(&lambda.body)
    }

    fn compile_let(&mut self, name: &tisp_core::symbol::Symbol, value: &CoreExpr, body: &CoreExpr) -> String {
        let value_ir = self.compile_expr(value);
        // Extract the result register from value_ir
        let val_reg = self.fresh_reg();
        self.locals.insert(name.as_str().to_string(), format!("%{}", val_reg));

        let body_ir = self.compile_expr(body);
        self.locals.remove(name.as_str());

        format!("{}\n{}", ret_to_assign(&value_ir, &format!("%{}", val_reg)), body_ir)
    }

    fn compile_if(&mut self, cond: &CoreExpr, then: &CoreExpr, else_: &CoreExpr) -> String {
        let cond_val = self.eval_to_reg(cond);
        let then_label = self.fresh_label();
        let else_label = self.fresh_label();
        let merge_label = self.fresh_label();
        let result = self.fresh_reg();

        let then_body = self.compile_expr(then);
        let else_body = self.compile_expr(else_);
        let phi_target = self.fresh_reg();

        format!(
            "  %{} = icmp ne i64 {}, 0\n  br i1 %{}, label %{}, label %{}\n\
             {}:\n{}\n  br label %{}\n\
             {}:\n{}\n  br label %{}\n\
             {}:\n  %{} = phi i64 [%tmp_then, {}], [%tmp_else, {}]\n  ret i64 %{}",
            result, cond_val, result, then_label, else_label,
            then_label, ret_to_assign(&then_body, "%tmp_then"), merge_label,
            else_label, ret_to_assign(&else_body, "%tmp_else"), merge_label,
            merge_label, phi_target, then_label, else_label, phi_target
        )
    }

    fn eval_to_reg(&mut self, expr: &CoreExpr) -> String {
        match &expr.node {
            CoreExprNode::Lit(lit) => {
                let reg = format!("%r{}", self.fresh_reg());
                let val = match lit {
                    Literal::I64(n) => format!("{}", n),
                    Literal::Bool(true) => "1".into(),
                    Literal::Bool(false) => "0".into(),
                    _ => "0".into(),
                };
                self.ir_buf.push_str(&format!("  {} = add i64 {}, 0\n", reg, val));
                reg
            }
            CoreExprNode::Var(name) => {
                self.locals.get(name.as_str()).cloned().unwrap_or_else(|| "%0".to_string())
            }
            _ => {
                // For complex expressions, compile and extract register
                let ir = self.compile_expr(expr);
                if let Some(rest) = ir.strip_prefix("  ret i64 %") {
                    rest.trim().to_string()
                } else {
                    "%0".to_string()
                }
            }
        }
    }

    fn fresh_reg(&mut self) -> usize {
        let r = self.next_reg; self.next_reg += 1; r
    }

    fn fresh_label(&mut self) -> String {
        let l = format!("L{}", self.next_label); self.next_label += 1; l
    }
}

/// inkwell 真实 IR 生成(仅 llvm feature)
#[cfg(feature = "llvm")]
fn llvm_generate(program: &CoreProgram) -> String {
    use inkwell::context::Context;
    use inkwell::values::{FunctionValue, IntValue};
    use inkwell::IntPredicate;
    use tisp_core::symbol::Symbol;

    #[allow(clippy::too_many_arguments)]
    fn compile_expr<'ctx>(
        context: &Context,
        builder: &inkwell::builder::Builder<'ctx>,
        i64_ty: &inkwell::types::IntType<'ctx>,
        func: FunctionValue<'ctx>,
        funcs: &HashMap<String, FunctionValue<'ctx>>,
        expr: &CoreExpr,
        env: &mut HashMap<String, IntValue<'ctx>>,
    ) -> IntValue<'ctx> {
        // 用户函数调用参数收集:(f a b) → f + [a, b]
        fn collect_args<'a>(expr: &'a CoreExpr, acc: &mut Vec<&'a CoreExpr>) -> Option<&'a Symbol> {
            match &expr.node {
                CoreExprNode::App(f, a) => {
                    if let Some(sym) = collect_args(f, acc) {
                        acc.push(a);
                        Some(sym)
                    } else {
                        None
                    }
                }
                CoreExprNode::Var(name) => Some(name),
                _ => None,
            }
        }

        match &expr.node {
            CoreExprNode::Lit(lit) => match lit {
                Literal::I64(n) => i64_ty.const_int(*n as u64, false),
                Literal::Bool(b) => i64_ty.const_int(*b as u64, false),
                _ => i64_ty.const_zero(),
            },
            CoreExprNode::Var(name) => env
                .get(name.as_str())
                .copied()
                .unwrap_or_else(|| i64_ty.const_zero()),
            CoreExprNode::App(func_expr, arg) => {
                // 二元运算/比较:(op a b) → App(App(Var(op), a), b)
                if let CoreExprNode::App(inner_func, inner_arg) = &func_expr.node {
                    if let CoreExprNode::Var(op_name) = &inner_func.node {
                        let lhs = compile_expr(context, builder, i64_ty, func, funcs, inner_arg, env);
                        let rhs = compile_expr(context, builder, i64_ty, func, funcs, arg, env);
                        // 比较结果(i1)统一零扩展为 i64,保证所有值同类型
                        let cmp = |pred: IntPredicate, name: &str| {
                            let c = builder.build_int_compare(pred, lhs, rhs, name).unwrap();
                            builder.build_int_z_extend(c, *i64_ty, name).unwrap()
                        };
                        return match op_name.as_str() {
                            "+" => builder.build_int_add(lhs, rhs, "add").unwrap(),
                            "-" => builder.build_int_sub(lhs, rhs, "sub").unwrap(),
                            "*" => builder.build_int_mul(lhs, rhs, "mul").unwrap(),
                            "/" => builder.build_int_signed_div(lhs, rhs, "div").unwrap(),
                            "<" => cmp(IntPredicate::SLT, "lt"),
                            ">" => cmp(IntPredicate::SGT, "gt"),
                            "<=" => cmp(IntPredicate::SLE, "le"),
                            ">=" => cmp(IntPredicate::SGE, "ge"),
                            "=" => cmp(IntPredicate::EQ, "eq"),
                            _ => lhs,
                        };
                    }
                }
                // 用户函数调用:(f a b ...),支持递归;collect_args 收集 func 链上的
                // 参数,最外层 arg 需补入
                let mut call_args: Vec<&CoreExpr> = vec![];
                if let Some(name) = collect_args(func_expr, &mut call_args) {
                    call_args.push(arg);
                    if let Some(callee) = funcs.get(name.as_str()) {
                        let arg_vals: Vec<inkwell::values::BasicMetadataValueEnum> = call_args
                            .iter()
                            .map(|a| compile_expr(context, builder, i64_ty, func, funcs, a, env).into())
                            .collect();
                        let call = builder
                            .build_call(*callee, &arg_vals, "call")
                            .unwrap();
                        if let Some(v) = call.try_as_basic_value().left() {
                            return v.into_int_value();
                        }
                    }
                }
                i64_ty.const_zero()
            }
            CoreExprNode::Lam(lambda) => {
                compile_expr(context, builder, i64_ty, func, funcs, &lambda.body, env)
            }
            CoreExprNode::Let(name, _, value, body) => {
                let v = compile_expr(context, builder, i64_ty, func, funcs, value, env);
                env.insert(name.as_str().to_string(), v);
                let r = compile_expr(context, builder, i64_ty, func, funcs, body, env);
                env.remove(name.as_str());
                r
            }
            CoreExprNode::If(cond, then, else_) => {
                let cond_val = compile_expr(context, builder, i64_ty, func, funcs, cond, env);
                let cond_bool = builder
                    .build_int_compare(IntPredicate::NE, cond_val, i64_ty.const_zero(), "cond")
                    .unwrap();
                let then_block = context.append_basic_block(func, "then");
                let else_block = context.append_basic_block(func, "else");
                let merge_block = context.append_basic_block(func, "merge");
                builder.build_conditional_branch(cond_bool, then_block, else_block).unwrap();

                builder.position_at_end(then_block);
                let then_val = compile_expr(context, builder, i64_ty, func, funcs, then, env);
                builder.build_unconditional_branch(merge_block).unwrap();

                builder.position_at_end(else_block);
                let else_val = compile_expr(context, builder, i64_ty, func, funcs, else_, env);
                builder.build_unconditional_branch(merge_block).unwrap();

                builder.position_at_end(merge_block);
                let phi = builder.build_phi(*i64_ty, "phi").unwrap();
                phi.add_incoming(&[(&then_val, then_block), (&else_val, else_block)]);
                phi.as_basic_value().into_int_value()
            }
            CoreExprNode::Do(exprs) => {
                let mut last = i64_ty.const_zero();
                for e in exprs {
                    last = compile_expr(context, builder, i64_ty, func, funcs, e, env);
                }
                last
            }
            _ => i64_ty.const_zero(),
        }
    }

    let context = Context::create();
    let module = context.create_module("tisp");
    let builder = context.create_builder();
    let i64_ty = context.i64_type();

    // 从 def body 提取函数参数(desugar 的 def body 是 Lam(params, body))
    let def_params: Vec<Vec<Symbol>> = program
        .defs
        .iter()
        .map(|def| match &def.body.node {
            CoreExprNode::Lam(l) => l.params.iter().map(|p| p.name.clone()).collect(),
            _ => vec![],
        })
        .collect();

    // 预声明全部 def(递归调用需要先有函数符号),签名含参数
    let mut funcs: HashMap<String, FunctionValue> = HashMap::new();
    for (def, params) in program.defs.iter().zip(def_params.iter()) {
        let fn_ty = i64_ty.fn_type(
            &(0..params.len())
                .map(|_| i64_ty.into())
                .collect::<Vec<inkwell::types::BasicMetadataTypeEnum>>(),
            false,
        );
        let f = module.add_function(def.name.as_str(), fn_ty, None);
        funcs.insert(def.name.as_str().to_string(), f);
    }

    for (def, params) in program.defs.iter().zip(def_params.iter()) {
        let func = funcs[def.name.as_str()];
        let entry = context.append_basic_block(func, "entry");
        builder.position_at_end(entry);
        let mut env: HashMap<String, IntValue> = HashMap::new();
        // 绑定函数参数
        for (i, p) in params.iter().enumerate() {
            if let Some(pv) = func.get_nth_param(i as u32) {
                env.insert(p.as_str().to_string(), pv.into_int_value());
            }
        }
        let val = compile_expr(&context, &builder, &i64_ty, func, &funcs, &def.body, &mut env);
        let _ = builder.build_return(Some(&val));
    }

    module.print_to_string().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tisp_core::span::Span;
    use tisp_core::symbol::Symbol;

    fn expr(node: CoreExprNode) -> CoreExpr {
        CoreExpr::new(node, Span::dummy())
    }

    fn def(name: &str, body: CoreExpr) -> CoreDef {
        CoreDef {
            name: Symbol::new(name),
            ty: None,
            effects: tisp_core::types::EffectRow::Pure,
            grade: tisp_core::types::Grade::Omega,
            mode: tisp_core::types::Mode::In,
            determinism: tisp_core::types::Determinism::Det,
            mode_sigs: vec![],
            body,
            requires: None,
            ensures: None,
            span: Span::dummy(),
        }
    }

    #[test]
    fn test_ir_function_header() {
        // §30:define i64 @main() { 语法正确
        let body = expr(CoreExprNode::Lit(Literal::I64(42)));
        let program = CoreProgram { data_decls: vec![], effect_decls: vec![], type_families: vec![],
            resource_algebras: vec![], defs: vec![def("main", body)] };
        let ir = IrGenerator::new().generate(&program);
        // 文本生成器以 define 开头;inkwell 输出带 ModuleID 头,统一断言包含定义
        assert!(ir.contains("define i64 @main() {"), "header malformed: {}", ir);
        assert!(ir.trim_end().ends_with("}"), "missing closing brace: {}", ir);
        assert!(ir.contains("ret i64"));
    }

    #[test]
    fn test_ir_arithmetic() {
        // (+ 21 21) → add 指令
        let add = expr(CoreExprNode::App(
            Box::new(expr(CoreExprNode::App(Box::new(expr(CoreExprNode::Var(Symbol::new("+")))), Box::new(expr(CoreExprNode::Lit(Literal::I64(21))))))),
            Box::new(expr(CoreExprNode::Lit(Literal::I64(21)))),
        ));
        let program = CoreProgram { data_decls: vec![], effect_decls: vec![], type_families: vec![],
            resource_algebras: vec![], defs: vec![def("main", add)] };
        let ir = IrGenerator::new().generate(&program);
        // LLVM IRBuilder 对常量操作数立即折叠(21+21 → 42,不发射 add 指令)
        #[cfg(feature = "llvm")]
        assert!(ir.contains("ret i64 42"), "expected folded constant: {}", ir);
        #[cfg(not(feature = "llvm"))]
        assert!(ir.contains("= add i64 "), "expected add instruction: {}", ir);
    }

    #[test]
    fn test_ir_phi_register_consistency() {
        // if 编译:phi 目标与 ret 引用同一寄存器
        let cond = expr(CoreExprNode::App(
            Box::new(expr(CoreExprNode::App(Box::new(expr(CoreExprNode::Var(Symbol::new(">")))), Box::new(expr(CoreExprNode::Lit(Literal::I64(1))))))),
            Box::new(expr(CoreExprNode::Lit(Literal::I64(0)))),
        ));
        let ife = expr(CoreExprNode::If(
            Box::new(cond),
            Box::new(expr(CoreExprNode::Lit(Literal::I64(1)))),
            Box::new(expr(CoreExprNode::Lit(Literal::I64(0)))),
        ));
        let program = CoreProgram { data_decls: vec![], effect_decls: vec![], type_families: vec![],
            resource_algebras: vec![], defs: vec![def("main", ife)] };
        let ir = IrGenerator::new().generate(&program);
        assert!(ir.contains("phi i64"), "expected phi: {}", ir);
        // 每处 ret i64 %N 引用的寄存器都有定义
        for line in ir.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("ret i64 %") {
                assert!(ir.contains(&format!("%{} = ", rest)), "ret references undefined %{}", rest);
            }
        }
    }
}
