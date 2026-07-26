/// Simple LLVM IR text generator (no inkwell dependency needed)
/// Generates human-readable LLVM IR files compilable with llc

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
                "define i64 @{}({{\nentry:\n{}{}}}\n",
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

        format!("{}\n{}", value_ir.replace("ret i64", &format!("%{} = add i64", val_reg)), body_ir)
    }

    fn compile_if(&mut self, cond: &CoreExpr, then: &CoreExpr, else_: &CoreExpr) -> String {
        let cond_val = self.eval_to_reg(cond);
        let then_label = self.fresh_label();
        let else_label = self.fresh_label();
        let merge_label = self.fresh_label();
        let result = self.fresh_reg();

        let then_body = self.compile_expr(then);
        let else_body = self.compile_expr(else_);

        format!(
            "  %{} = icmp ne i64 {}, 0\n  br i1 %{}, label %{}, label %{}\n\
             {}:\n{}\n  br label %{}\n\
             {}:\n{}\n  br label %{}\n\
             {}:\n  %{} = phi i64 [%tmp_then, {}], [%tmp_else, {}]\n  ret i64 %{}",
            result, cond_val, result, then_label, else_label,
            then_label, then_body.replace("ret i64", "%tmp_then = add i64"), merge_label,
            else_label, else_body.replace("ret i64", "%tmp_else = add i64"), merge_label,
            merge_label, self.fresh_reg(), then_label, else_label, self.fresh_reg()
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
