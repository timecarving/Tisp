use crate::symbol::Symbol;
use crate::types::{EffectRow, EffectLabel, Type};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EffectDecl {
    pub name: Symbol,
    pub type_params: Vec<Symbol>,
    pub operations: Vec<OperationDecl>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OperationDecl {
    pub name: Symbol,
    pub params: Vec<Type>,
    pub return_type: Type,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Handler {
    pub effect_name: Symbol,
    pub type_args: Vec<Type>,
    pub clauses: Vec<HandlerClause>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HandlerClause {
    pub operation: Symbol,
    pub params: Vec<Symbol>,
    pub continuation_vars: Vec<Symbol>,
    pub body: crate::ast::SExpr,
}

pub fn row_union(a: &EffectRow, b: &EffectRow) -> EffectRow {
    match (a, b) {
        (EffectRow::Pure, r) | (r, EffectRow::Pure) => r.clone(),
        (EffectRow::Closed(xs), EffectRow::Closed(ys)) => {
            let mut merged = xs.clone();
            for y in ys {
                if !merged.contains(y) {
                    merged.push(y.clone());
                }
            }
            EffectRow::Closed(merged)
        }
        _ => EffectRow::Var(0),
    }
}

pub fn row_contains(row: &EffectRow, label: &EffectLabel) -> bool {
    match row {
        EffectRow::Pure => false,
        EffectRow::Closed(labels) => labels.contains(label),
        EffectRow::Open(labels, _) => labels.contains(label),
        EffectRow::Var(_) => true,
    }
}
