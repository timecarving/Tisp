use tisp_core::core_ast::*;
use tisp_core::symbol::Symbol;
use tisp_core::regions::*;
use tisp_core::span::Span;
use std::collections::HashMap;

pub struct RegionInfer { next_region_id: u64, allocation_regions: HashMap<usize, Region>, region_names: HashMap<String, u64> }
#[derive(Debug, Clone)] pub struct RegionError { pub message: String, pub span: Span }
impl std::fmt::Display for RegionError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "region error: {} at {}", self.message, self.span) } }
impl std::error::Error for RegionError {}

impl RegionInfer {
    pub fn new() -> Self { Self { next_region_id: 0, allocation_regions: HashMap::new(), region_names: HashMap::new() } }
    pub fn infer_program(&mut self, prog: &CoreProgram) -> Result<Vec<(Symbol, Vec<Region>)>, RegionError> {
        let mut results = Vec::new();
        for def in &prog.defs { results.push((def.name.clone(), self.infer_def(def)?)); }
        Ok(results)
    }
    fn infer_def(&mut self, def: &CoreDef) -> Result<Vec<Region>, RegionError> {
        self.allocation_regions.clear();
        self.walk(&def.body)?;
        // §26.3 编译期区域逃逸检查:分配的地址作为返回值逃出作用域 → 报错
        if self.check_escape(&def.body) {
            return Err(RegionError {
                message: format!("区域逃逸:定义 '{}' 的分配地址作为返回值逃出区域作用域", def.name),
                span: def.span,
            });
        }
        let mut regions = Vec::new();
        for r in self.allocation_regions.values() { if !regions.contains(r) { regions.push(r.clone()); } }
        Ok(regions)
    }
    /// 判断表达式是否把 `RegionAlloc` 的分配地址作为最终值(逃逸)。
    /// 完整数据流:跟踪 Let 绑定到 `RegionAlloc` 的地址名,若该名在尾位置返回则逃逸;
    /// Do/Let 追尾;if 两分支均逃逸才算逃逸。
    fn check_escape(&self, expr: &CoreExpr) -> bool {
        let mut allocs = std::collections::HashSet::new();
        self.escape_walk(expr, &mut allocs)
    }

    fn escape_walk(&self, expr: &CoreExpr, allocs: &mut std::collections::HashSet<Symbol>) -> bool {
        match &expr.node {
            CoreExprNode::RegionAlloc(_, _) => true,
            CoreExprNode::Var(name) => allocs.contains(name),
            CoreExprNode::Do(es) => es.last().map_or(false, |e| self.escape_walk(e, allocs)),
            CoreExprNode::Let(name, _, value, body) => {
                // 绑定到 RegionAlloc 的地址名标记为已分配(数据流逃逸跟踪)
                if matches!(&value.node, CoreExprNode::RegionAlloc(_, _)) {
                    allocs.insert(name.clone());
                }
                self.escape_walk(body, allocs)
            }
            CoreExprNode::If(_, t, e) => self.escape_walk(t, allocs) && self.escape_walk(e, allocs),
            // 完整别名分析:闭包捕获已分配地址 → 逃逸(地址被闭包带出作用域)
            CoreExprNode::Lam(lambda) => uses_allocated(&lambda.body, allocs),
            // 完整别名分析:地址作为实参流入函数 → 可能逃逸(保守报)
            CoreExprNode::App(f, a) => uses_allocated(f, allocs) || uses_allocated(a, allocs),
            // 跨区域/全局别名:地址嵌入数据结构(Data)→ 堆逃逸
            CoreExprNode::Data(_, args) => args.iter().any(|a| uses_allocated(a, allocs)),
            // 跨区域/全局别名:地址在 match 分支中使用 → 逃逸
            CoreExprNode::Match(s, arms) => {
                uses_allocated(s, allocs)
                    || arms.iter().any(|arm| uses_allocated(&arm.body, allocs))
            }
            _ => false,
        }
    }
    fn walk(&mut self, expr: &CoreExpr) -> Result<(), RegionError> {
        match &expr.node {
            CoreExprNode::Lit(_)|CoreExprNode::Var(_)|CoreExprNode::Hole(_) => Ok(()),
            CoreExprNode::Do(es) => { for e in es { self.walk(e)?; } Ok(()) }
            CoreExprNode::Lam(l) => { self.alloc(hash_expr(expr), "closure"); self.walk(&l.body) }
            CoreExprNode::App(f,a) => { self.walk(f)?; self.walk(a)?; Ok(()) }
            CoreExprNode::Let(_,_,v,b) => { self.walk_alloc(v)?; self.walk(b) }
            CoreExprNode::If(c,t,e) => { self.walk(c)?; self.walk(t)?; self.walk(e)?; Ok(()) }
            CoreExprNode::Match(s,arms) => {
                self.walk(s)?;
                for arm in arms { if let Some(g)=&arm.guard { self.walk(g)?; } self.walk(&arm.body)?; }
                Ok(())
            }
            CoreExprNode::Data(_,args) => { self.alloc(hash_expr(expr), "data"); for a in args { self.walk(a)?; } Ok(()) }
            CoreExprNode::Handle(b,_) => self.walk(b),
            CoreExprNode::Perform(_,args) => { for a in args { self.walk(a)?; } Ok(()) }
            _ => Ok(()),
        }
    }
    fn walk_alloc(&mut self, expr: &CoreExpr) -> Result<(), RegionError> {
        match &expr.node { CoreExprNode::Data(..)|CoreExprNode::Lam(_) => { self.alloc(hash_expr(expr), "value"); } _=>{} }
        self.walk(expr)
    }
    fn alloc(&mut self, key: usize, prefix: &str) {
        let id = self.next_region_id; self.next_region_id += 1;
        let c = self.region_names.entry(prefix.to_string()).or_insert(0); *c += 1;
        self.allocation_regions.insert(key, Region::Var(RegionId { name: Symbol::new(&format!("ρ_{}{}", prefix, c)), id }));
    }
    pub fn classify_regions(&self) -> HashMap<u64, RegionInfo> {
        let mut info = HashMap::new();
        for r in self.allocation_regions.values() {
            if let Region::Var(id) = r {
                let c = self.allocation_regions.values().filter(|v| matches!(v, Region::Var(o) if o.id == id.id)).count();
                info.insert(id.id, RegionInfo { id: id.clone(), kind: if c <= 1 { RegionKind::Scalar } else { RegionKind::Finite }, multiplicity: if c <= 1 { RegionMultiplicity::One } else { RegionMultiplicity::Infinite }, runtime_type: RuntimeType::Top });
            }
        }
        info
    }
}
fn hash_expr(expr: &CoreExpr) -> usize { std::ptr::from_ref(expr) as usize }

/// 判断表达式是否引用了任一「已分配地址名」(用于闭包捕获/实参流入的别名逃逸)
fn uses_allocated(expr: &CoreExpr, allocs: &std::collections::HashSet<Symbol>) -> bool {
    match &expr.node {
        CoreExprNode::Var(name) => allocs.contains(name),
        CoreExprNode::App(f, a) => uses_allocated(f, allocs) || uses_allocated(a, allocs),
        CoreExprNode::Let(_, _, v, b) => uses_allocated(v, allocs) || uses_allocated(b, allocs),
        CoreExprNode::If(c, t, e) => uses_allocated(c, allocs) || uses_allocated(t, allocs) || uses_allocated(e, allocs),
        CoreExprNode::Lam(l) => uses_allocated(&l.body, allocs),
        CoreExprNode::Do(es) => es.iter().any(|e| uses_allocated(e, allocs)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tisp_core::types::{EffectRow, Grade, Mode, Determinism};

    fn e(node: CoreExprNode) -> CoreExpr {
        CoreExpr::new(node, Span::dummy())
    }

    fn def_with_body(body: CoreExpr) -> CoreDef {
        CoreDef {
            name: Symbol::new("f"),
            ty: None,
            effects: EffectRow::Pure,
            grade: Grade::Omega,
            mode: Mode::In,
            region: None,
            visibility: Visibility::Public,
            mode_sigs: vec![],
            determinism: Determinism::Det,
            body,
            requires: None,
            ensures: None,
            span: Span::dummy(),
        }
    }

    #[test]
    fn test_region_escape_detected() {
        // 分配地址作为函数返回值 → 逃逸
        let body = e(CoreExprNode::RegionAlloc(
            Box::new(e(CoreExprNode::Lit(Literal::Unit))),
            Box::new(e(CoreExprNode::Lit(Literal::I64(42)))),
        ));
        let mut inf = RegionInfer::new();
        let r = inf.infer_def(&def_with_body(body));
        assert!(r.is_err(), "分配地址逃逸应报错");
        assert!(r.unwrap_err().message.contains("区域逃逸"));
    }

    #[test]
    fn test_no_escape_for_literal() {
        // 普通字面量返回 → 不逃逸
        let body = e(CoreExprNode::Lit(Literal::I64(42)));
        let mut inf = RegionInfer::new();
        assert!(inf.infer_def(&def_with_body(body)).is_ok());
    }

    #[test]
    fn test_region_escape_via_let_binding() {
        // 数据流逃逸:(let [r (region-alloc ...)] r) → 分配的地址经 let 绑定返回 → 逃逸
        let body = e(CoreExprNode::Let(
            Symbol::new("r"),
            None,
            Box::new(e(CoreExprNode::RegionAlloc(
                Box::new(e(CoreExprNode::Lit(Literal::Unit))),
                Box::new(e(CoreExprNode::Lit(Literal::I64(42)))),
            ))),
            Box::new(e(CoreExprNode::Var(Symbol::new("r")))),
        ));
        let mut inf = RegionInfer::new();
        let r = inf.infer_def(&def_with_body(body));
        assert!(r.is_err(), "let 绑定的分配地址返回应报逃逸");
    }

    #[test]
    fn test_region_escape_via_closure_capture() {
        // 完整别名分析:闭包捕获已分配地址 → 逃逸
        let body = e(CoreExprNode::Let(
            Symbol::new("r"),
            None,
            Box::new(e(CoreExprNode::RegionAlloc(
                Box::new(e(CoreExprNode::Lit(Literal::Unit))),
                Box::new(e(CoreExprNode::Lit(Literal::I64(42)))),
            ))),
            Box::new(e(CoreExprNode::Lam(Lambda {
                params: vec![],
                body: Box::new(e(CoreExprNode::Var(Symbol::new("r")))),
                ret_type: None,
            }))),
        ));
        let mut inf = RegionInfer::new();
        let r = inf.infer_def(&def_with_body(body));
        assert!(r.is_err(), "闭包捕获分配地址应报逃逸");
    }

    #[test]
    fn test_region_escape_via_data_embed() {
        // 跨区域/全局别名:地址嵌入数据结构(Data)→ 堆逃逸
        let body = e(CoreExprNode::Let(
            Symbol::new("r"),
            None,
            Box::new(e(CoreExprNode::RegionAlloc(
                Box::new(e(CoreExprNode::Lit(Literal::Unit))),
                Box::new(e(CoreExprNode::Lit(Literal::I64(42)))),
            ))),
            Box::new(e(CoreExprNode::Data(Symbol::new("Cons"), vec![
                e(CoreExprNode::Var(Symbol::new("r"))),
                e(CoreExprNode::Lit(Literal::Unit)),
            ]))),
        ));
        let mut inf = RegionInfer::new();
        let r = inf.infer_def(&def_with_body(body));
        assert!(r.is_err(), "地址嵌入数据结构应报逃逸");
    }
}
