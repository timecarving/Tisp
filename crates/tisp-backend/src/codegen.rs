/// Simple LLVM IR text generator (no inkwell dependency needed)
/// Generates human-readable LLVM IR files compilable with llc

use tisp_core::core_ast::*;
use tisp_core::types::Type;
use std::collections::HashMap;

/// 把 IR 中最后一条 "ret i64 %X" 转成 "target = add i64 %X, 0"(支持多行)
fn ret_to_assign(ir: &str, target: &str) -> String {
    if let Some(idx) = ir.rfind("  ret i64 ") {
        let (head, tail) = ir.split_at(idx);
        let val = tail.trim_start().trim_start_matches("ret i64 ").trim();
        format!("{}{} = add i64 {}, 0", head, target, val)
    } else {
        ir.to_string()
    }
}

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

pub struct IrGenerator {
    next_label: usize,
    next_reg: usize,
    locals: HashMap<String, String>,
    /// Accumulated IR text from val_to_reg calls
    ir_buf: String,
}

#[allow(dead_code)]
struct IrFunction;

impl IrGenerator {
    pub fn new() -> Self {
        Self { next_label: 0, next_reg: 0, locals: HashMap::new(), ir_buf: String::new() }
    }

    pub fn generate(&mut self, program: &CoreProgram) -> String {
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
        let program = CoreProgram { data_decls: vec![], effect_decls: vec![], defs: vec![def("main", body)] };
        let ir = IrGenerator::new().generate(&program);
        assert!(ir.starts_with("define i64 @main() {\nentry:\n"), "header malformed: {}", ir);
        assert!(ir.ends_with("}\n"), "missing closing brace");
        assert!(ir.contains("ret i64"));
    }

    #[test]
    fn test_ir_arithmetic() {
        // (+ 21 21) → add 指令
        let add = expr(CoreExprNode::App(
            Box::new(expr(CoreExprNode::App(Box::new(expr(CoreExprNode::Var(Symbol::new("+")))), Box::new(expr(CoreExprNode::Lit(Literal::I64(21))))))),
            Box::new(expr(CoreExprNode::Lit(Literal::I64(21)))),
        ));
        let program = CoreProgram { data_decls: vec![], effect_decls: vec![], defs: vec![def("main", add)] };
        let ir = IrGenerator::new().generate(&program);
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
        let program = CoreProgram { data_decls: vec![], effect_decls: vec![], defs: vec![def("main", ife)] };
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
