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
        let mut regions = Vec::new();
        for r in self.allocation_regions.values() { if !regions.contains(r) { regions.push(r.clone()); } }
        Ok(regions)
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
