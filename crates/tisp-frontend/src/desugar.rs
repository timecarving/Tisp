use tisp_core::ast::{Expr, SExpr};
use tisp_core::core_ast::*;
use tisp_core::span::Span;
use tisp_core::symbol::Symbol;
use tisp_core::types::{Grade, Mode, EffectRow, EffectLabel, Determinism, Predicate, CmpOp, BinOp, Lit};
use tisp_core::data::{DataDecl, Constructor, Field};

#[derive(Debug, Clone)]
pub struct DesugarError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for DesugarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {}", self.message, self.span)
    }
}

impl std::error::Error for DesugarError {}

#[allow(dead_code)]
enum TopLevel {
    DataDecl(DataDecl),
    Def(CoreDef),
    EffectDecl(tisp_core::effects::EffectDecl),
    Namespace(Symbol, Vec<(Symbol, Symbol)>, Vec<Symbol>),
    FFIDecl(Symbol, String, Vec<tisp_core::types::Type>, Option<tisp_core::types::Type>, Vec<tisp_core::types::EffectLabel>),
}

pub struct Desugarer;

impl Desugarer {
    pub fn new() -> Self {
        Self
    }

    pub fn desugar_program(&self, forms: Vec<SExpr>) -> Result<CoreProgram, DesugarError> {
        let mut data_decls = Vec::new();
        let mut effect_decls = Vec::new();
        let mut defs = Vec::new();
        for form in forms {
            match self.desugar_top_level(&form)? {
                Some(TopLevel::DataDecl(decl)) => data_decls.push(decl),
                Some(TopLevel::EffectDecl(decl)) => effect_decls.push(decl),
                Some(TopLevel::Def(def)) => defs.push(def),
                Some(TopLevel::Namespace(name, _, _)) => {
                    defs.push(CoreDef { name: name.clone(), ty: None, effects: EffectRow::Pure, grade: Grade::Omega,
                        mode: Mode::In, determinism: Determinism::Det,
                        body: CoreExpr::new(CoreExprNode::NSDef(name, vec![], vec![]), Span::dummy()),
                        requires: None, ensures: None, span: Span::dummy() });
                }
                Some(TopLevel::FFIDecl(name, c_name, params, ret, effects)) => {
                    defs.push(CoreDef { name: name.clone(), ty: None, effects: EffectRow::Closed(effects), grade: Grade::Omega,
                        mode: Mode::In, determinism: Determinism::Det,
                        body: CoreExpr::new(CoreExprNode::ExternDef(name, c_name, params, ret, vec![]), Span::dummy()),
                        requires: None, ensures: None, span: Span::dummy() });
                }
                None => {}
            }
        }
        Ok(CoreProgram { data_decls, effect_decls, defs })
    }

    fn desugar_top_level(&self, expr: &SExpr) -> Result<Option<TopLevel>, DesugarError> {
        match &expr.node {
            Expr::List(items) if !items.is_empty() => {
                if let Expr::Sym(name) = &items[0].node {
                    match name.as_str() {
                        "def" => return Ok(Some(TopLevel::Def(self.desugar_def_form(items, expr.span)?))),
                        "defn" => return Ok(Some(TopLevel::Def(self.desugar_defn_form(items, expr.span)?))),
                        "defn-" => return Ok(Some(TopLevel::Def(self.desugar_defn_form(items, expr.span)?))),
                        "def-" => return Ok(Some(TopLevel::Def(self.desugar_def_form(items, expr.span)?))),
                        "defdata" => return Ok(Some(TopLevel::DataDecl(self.desugar_defdata_form(items, expr.span)?))),
                        "defdata-hit" => return Ok(Some(TopLevel::DataDecl(self.desugar_defdata_hit_form(items, expr.span)?))),
                        "defeffect" => {
                            return Ok(Some(TopLevel::EffectDecl(self.desugar_defeffect_form(items, expr.span)?)));
                        }
                        "defpred" => return Ok(Some(TopLevel::Def(self.desugar_defpred_form(items, expr.span)?))),
                        "defclass" => return self.desugar_stub_defn(items, "defclass", expr.span),
                        "definstance" => return self.desugar_stub_defn(items, "definstance", expr.span),
                        "defgeneric" => return self.desugar_stub_defn(items, "defgeneric", expr.span),
                        "defmethod" => return self.desugar_stub_defn(items, "defmethod", expr.span),
                        "defmacro" => return self.desugar_stub_defn(items, "defmacro", expr.span),
                        "defextern" => return self.desugar_defextern_form(items, expr.span),
                        "defresource-algebra" => return self.desugar_stub_defn(items, "defresource-algebra", expr.span),
                        "defsession" => return self.desugar_defsession_form(items, expr.span),
                        "ns" => return self.desugar_ns_form(items, expr.span),
                        _ => return Ok(None),
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn desugar_stub_defn(&self, items: &[SExpr], _tag: &str, _span: Span) -> Result<Option<TopLevel>, DesugarError> {
        let name = match items.get(1).and_then(|i| match &i.node { Expr::Sym(s) => Some(s.clone()), _ => None }) {
            Some(s) => s,
            None => return Ok(None),
        };
        let body = CoreExpr::new(CoreExprNode::Lit(Literal::Unit), _span);
        let def = CoreDef { name, ty: None, effects: EffectRow::Pure, grade: Grade::Omega,
            mode: Mode::In, determinism: Determinism::Det, body, requires: None, ensures: None, span: _span };
        Ok(Some(TopLevel::Def(def)))
    }

    fn desugar_ns_form(&self, items: &[SExpr], _span: Span) -> Result<Option<TopLevel>, DesugarError> {
        let name = match items.get(1).and_then(|i| match &i.node { Expr::Sym(s) => Some(s.clone()), _ => None }) {
            Some(s) => s,
            None => return Ok(None),
        };
        let mut requires = Vec::new();
        let mut refers = Vec::new();
        for item in &items[2..] {
            if let Expr::Keyword(kw) = &item.node {
                let tag = kw.as_str();
                if let Some(next) = items.iter().skip_while(|x| x.span != item.span).nth(1) {
                    if let Expr::Vec(v) = &next.node {
                        for entry in v {
                            if let Expr::Sym(m) = &entry.node {
                                if tag == "require" { requires.push((m.clone(), m.clone())); }
                                if tag == "refer" { refers.push(m.clone()); }
                            } else if let Expr::List(l) = &entry.node {
                                if tag == "require" && l.len() >= 2 {
                                    if let (Expr::Sym(m), Expr::Keyword(a)) = (&l[0].node, &l[1].node) {
                                        if a.as_str() == "as" && l.len() >= 3 {
                                            if let Expr::Sym(alias) = &l[2].node {
                                                requires.push((m.clone(), alias.clone()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(Some(TopLevel::Namespace(name, requires, refers)))
    }

    fn desugar_defextern_form(&self, items: &[SExpr], _span: Span) -> Result<Option<TopLevel>, DesugarError> {
        if items.len() < 4 { return Ok(None); }
        let name = match &items[1].node { Expr::Sym(s) => s.clone(), _ => return Ok(None) };
        let c_name = match &items[2].node { Expr::Str(s) => s.clone(), _ => return Ok(None) };
        Ok(Some(TopLevel::FFIDecl(name, c_name, vec![], None, vec![])))
    }
    fn desugar_defdata_hit_form(&self, items: &[SExpr], span: Span) -> Result<DataDecl, DesugarError> {
        let mut decl = self.desugar_defdata_form(items, span)?;
        decl.is_hit = true;
        Ok(decl)
    }

    fn desugar_defsession_form(&self, items: &[SExpr], span: Span) -> Result<Option<TopLevel>, DesugarError> {
        let name = match items.get(1).and_then(|i| match &i.node { Expr::Sym(s) => Some(s.clone()), _ => None }) {
            Some(s) => s,
            None => return Ok(None),
        };
        let mut ops = Vec::new();
        for item in &items[2..] {
            if let Expr::Sym(s) = &item.node { ops.push(s.clone()); }
        }
        let body = CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span);
        let def = CoreDef { name, ty: None, effects: EffectRow::Closed(vec![EffectLabel::Session]),
            grade: Grade::Omega, mode: Mode::In, determinism: Determinism::Det,
            body, requires: None, ensures: None, span };
        Ok(Some(TopLevel::Def(def)))
    }

    fn desugar_defdata_form(&self, items: &[SExpr], span: Span) -> Result<DataDecl, DesugarError> {
        if items.len() < 3 {
            return Err(DesugarError {
                message: "defdata requires name and at least one constructor".into(),
                span,
            });
        }

        // Parse name (possibly with type parameters)
        let (name, type_params) = match &items[1].node {
            Expr::Sym(s) => (s.clone(), Vec::new()),
            Expr::List(name_items) if !name_items.is_empty() => {
                if let Expr::Sym(s) = &name_items[0].node {
                    let params = name_items[1..].iter()
                        .map(|item| match &item.node {
                            Expr::Sym(p) => Ok(p.clone()),
                            _ => Err(DesugarError {
                                message: "type parameter must be a symbol".into(),
                                span: item.span,
                            }),
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    (s.clone(), params)
                } else {
                    return Err(DesugarError {
                        message: "defdata name must be a symbol".into(),
                        span: items[1].span,
                    });
                }
            }
            _ => return Err(DesugarError {
                message: "defdata name must be a symbol or (name params...)".into(),
                span: items[1].span,
            }),
        };

        // Parse constructors and :deriving
        let mut constructors = Vec::new();
        let mut deriving = Vec::new();
        let mut i = 2;
        while i < items.len() {
            if let Expr::Keyword(kw) = &items[i].node {
                if kw.as_str() == "deriving" && i + 1 < items.len() {
                    if let Expr::Vec(traits) = &items[i + 1].node {
                        for t in traits {
                            if let Expr::Sym(s) = &t.node { deriving.push(s.clone()); }
                        }
                    }
                    i += 2;
                    continue;
                }
            }
            constructors.push(self.desugar_constructor(&items[i], &type_params)?);
            i += 1;
        }

        Ok(DataDecl {
            name,
            type_params,
            constructors,
            deriving,
            is_hit: false,
            span,
        })
    }

    fn desugar_constructor(&self, expr: &SExpr, type_params: &[Symbol]) -> Result<Constructor, DesugarError> {
        match &expr.node {
            Expr::Sym(name) => {
                // Constructor with no fields
                Ok(Constructor {
                    name: name.clone(),
                    fields: Vec::new(),
                    gadt_return_type: None,
                    span: expr.span,
                })
            }
            Expr::List(items) if !items.is_empty() => {
                if let Expr::Sym(name) = &items[0].node {
                    let mut fields = Vec::new();
                    let mut gadt_return = None;
                    let mut i = 1;
                    while i < items.len() {
                        if let Expr::Keyword(kw) = &items[i].node {
                            if kw.as_str() == "->" && i + 1 < items.len() {
                                gadt_return = Some(self.desugar_type_with_params(&items[i + 1], type_params)?);
                                i += 2;
                                continue;
                            }
                        }
                        fields.push(self.desugar_field(&items[i], type_params)?);
                        i += 1;
                    }
                    Ok(Constructor {
                        name: name.clone(),
                        fields,
                        gadt_return_type: gadt_return,
                        span: expr.span,
                    })
                } else {
                    Err(DesugarError {
                        message: "constructor name must be a symbol".into(),
                        span: items[0].span,
                    })
                }
            }
            _ => Err(DesugarError {
                message: "invalid constructor syntax".into(),
                span: expr.span,
            }),
        }
    }

    fn desugar_field(&self, expr: &SExpr, type_params: &[Symbol]) -> Result<Field, DesugarError> {
        match &expr.node {
            Expr::Sym(name) => {
                // Check if this is a type parameter or built-in type
                if type_params.contains(name) || Self::is_builtin_type(name.as_str()) {
                    // Anonymous field with this type
                    let ty = self.desugar_type_with_params(expr, type_params)?;
                    Ok(Field { name: None, ty })
                } else {
                    // Field with no type annotation (will be inferred)
                    Ok(Field {
                        name: Some(name.clone()),
                        ty: tisp_core::types::Type::Var(tisp_core::types::TypeVar {
                            name: Symbol::new("_"),
                            kind: tisp_core::types::Kind::Star,
                            id: 0,
                        }),
                    })
                }
            }
            Expr::List(items) if !items.is_empty() => {
                // Could be either:
                // 1. (name Type) - named field with type annotation
                // 2. (Type arg1 arg2 ...) - anonymous field with type application
                
                // Check if first element is a symbol that could be a field name
                if let Expr::Sym(first) = &items[0].node {
                    // If it's a type parameter or built-in type, treat as type application
                    if type_params.contains(first) || Self::is_builtin_type(first.as_str()) {
                        let ty = self.desugar_type_with_params(expr, type_params)?;
                        Ok(Field { name: None, ty })
                    } else if items.len() == 2 {
                        // Could be (name Type) or (Type arg)
                        // If the first symbol is NOT a type parameter and NOT a built-in type,
                        // it's likely a type constructor, so treat as type application
                        match &items[1].node {
                            Expr::Sym(second) if type_params.contains(second) || Self::is_builtin_type(second.as_str()) => {
                                // If first is also a type param or builtin, it's (Type arg)
                                // Otherwise, it could be (name Type)
                                // Heuristic: if first starts with uppercase, it's likely a type constructor
                                if first.as_str().chars().next().map_or(false, |c| c.is_uppercase()) {
                                    let ty = self.desugar_type_with_params(expr, type_params)?;
                                    Ok(Field { name: None, ty })
                                } else {
                                    let ty = self.desugar_type_with_params(&items[1], type_params)?;
                                    Ok(Field { name: Some(first.clone()), ty })
                                }
                            }
                            Expr::List(_) => {
                                // (name (Type ...)) - named field with type application
                                let ty = self.desugar_type_with_params(&items[1], type_params)?;
                                Ok(Field { name: Some(first.clone()), ty })
                            }
                            _ => {
                                // Treat as type application
                                let ty = self.desugar_type_with_params(expr, type_params)?;
                                Ok(Field { name: None, ty })
                            }
                        }
                    } else {
                        // Multiple elements, treat as type application
                        let ty = self.desugar_type_with_params(expr, type_params)?;
                        Ok(Field { name: None, ty })
                    }
                } else {
                    // First element is not a symbol, treat as type application
                    let ty = self.desugar_type_with_params(expr, type_params)?;
                    Ok(Field { name: None, ty })
                }
            }
            _ => Err(DesugarError {
                message: "invalid field syntax".into(),
                span: expr.span,
            }),
        }
    }

    fn is_builtin_type(name: &str) -> bool {
        matches!(name, "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | 
                       "f32" | "f64" | "bool" | "String" | "Unit")
    }

    fn desugar_type_with_params(&self, expr: &SExpr, type_params: &[Symbol]) -> Result<tisp_core::types::Type, DesugarError> {
        match &expr.node {
            Expr::Sym(name) => {
                // Check if this is a type parameter
                if type_params.contains(name) {
                    Ok(tisp_core::types::Type::Var(tisp_core::types::TypeVar {
                        name: name.clone(),
                        kind: tisp_core::types::Kind::Star,
                        id: 0,
                    }))
                } else {
                    match name.as_str() {
                        "i8" => Ok(tisp_core::types::Type::i8()),
                        "i16" => Ok(tisp_core::types::Type::i16()),
                        "i32" => Ok(tisp_core::types::Type::i32()),
                        "i64" => Ok(tisp_core::types::Type::i64()),
                        "u8" => Ok(tisp_core::types::Type::u8()),
                        "u16" => Ok(tisp_core::types::Type::u16()),
                        "u32" => Ok(tisp_core::types::Type::u32()),
                        "u64" => Ok(tisp_core::types::Type::u64()),
                        "f32" => Ok(tisp_core::types::Type::f32()),
                        "f64" => Ok(tisp_core::types::Type::f64()),
                        "bool" => Ok(tisp_core::types::Type::bool()),
                        "String" => Ok(tisp_core::types::Type::string()),
                        "Unit" => Ok(tisp_core::types::Type::unit()),
                        _ => Ok(tisp_core::types::Type::Con(tisp_core::types::TypeCon {
                            name: name.clone(),
                            kind: tisp_core::types::Kind::Star,
                        })),
                    }
                }
            }
            // Refinement type: {name : Type | predicate}
            Expr::Map(pairs) if !pairs.is_empty() => {
                self.desugar_refined_type(expr, pairs, type_params)
            }
            Expr::List(items) if !items.is_empty() => {
                // Type application: (List i32), (Map String i32), etc.
                let base = self.desugar_type_with_params(&items[0], type_params)?;
                let mut result = base;
                for item in &items[1..] {
                    let arg = self.desugar_type_with_params(item, type_params)?;
                    result = tisp_core::types::Type::App(Box::new(result), Box::new(arg));
                }
                Ok(result)
            }
            _ => Err(DesugarError {
                message: "invalid type syntax".into(),
                span: expr.span,
            }),
        }
    }

    fn desugar_refined_type(&self, _expr: &SExpr, pairs: &[(SExpr, SExpr)], type_params: &[Symbol]) -> Result<tisp_core::types::Type, DesugarError> {
        // Parse {name : baseType | predicate}
        for (key, val) in pairs {
            if let Expr::Sym(_name) = &key.node {
                if let Expr::List(val_items) = &val.node {
                    if val_items.len() >= 2 {
                        // Last element is the predicate, items before are the base type
                        // But we need to find the | separator
                        for j in 1..val_items.len() {
                            if matches!(&val_items[j].node, Expr::Sym(s) if s.as_str() == "|") {
                                let type_items = &val_items[..j];
                                let pred_items = &val_items[j+1..];
                                
                                let base_type = if type_items.is_empty() {
                                    return Err(DesugarError {
                                        message: "refinement type missing base type".into(),
                                        span: key.span,
                                    });
                                } else if type_items.len() == 1 {
                                    self.desugar_type_with_params(&type_items[0], type_params)?
                                } else {
                                    let mut result = self.desugar_type_with_params(&type_items[0], type_params)?;
                                    for item in &type_items[1..] {
                                        let arg = self.desugar_type_with_params(item, type_params)?;
                                        result = tisp_core::types::Type::App(Box::new(result), Box::new(arg));
                                    }
                                    result
                                };

                                let predicate = if pred_items.is_empty() {
                                    return Err(DesugarError {
                                        message: "refinement type missing predicate".into(),
                                        span: key.span,
                                    });
                                } else if pred_items.len() == 1 {
                                    self.desugar_predicate(&pred_items[0])?
                                } else {
                                    let mut preds = Vec::new();
                                    for item in pred_items {
                                        preds.push(self.desugar_predicate(item)?);
                                    }
                                    let mut result = preds.remove(0);
                                    for p in preds {
                                        result = Predicate::And(Box::new(result), Box::new(p));
                                    }
                                    result
                                };

                                return Ok(tisp_core::types::Type::Refined(
                                    Box::new(base_type),
                                    Box::new(predicate),
                                ));
                            }
                        }
                        return Err(DesugarError {
                            message: "refinement type missing '|' separator".into(),
                            span: key.span,
                        });
                    }
                }
            }
        }
        Err(DesugarError {
            message: "invalid refinement type syntax, expected {name : Type | predicate}".into(),
            span: Span::dummy(),
        })
    }

    fn desugar_predicate(&self, expr: &SExpr) -> Result<Predicate, DesugarError> {
        match &expr.node {
            Expr::Sym(name) => {
                match name.as_str() {
                    "true" => Ok(Predicate::Lit(true)),
                    "false" => Ok(Predicate::Lit(false)),
                    _ => Ok(Predicate::Var(name.clone())),
                }
            }
            Expr::Int(n) => {
                Ok(Predicate::App(Symbol::new(&n.to_string()), vec![]))
            }
            Expr::Float(_) => {
                Ok(Predicate::Lit(true)) // placeholder
            }
            Expr::List(items) if !items.is_empty() => {
                if let Expr::Sym(op_name) = &items[0].node {
                    match op_name.as_str() {
                        "!" | "not" => {
                            if items.len() != 2 {
                                return Err(DesugarError { message: "not takes 1 arg".into(), span: expr.span });
                            }
                            Ok(Predicate::Not(Box::new(self.desugar_predicate(&items[1])?)))
                        }
                        "and" | "&&" => {
                            let mut preds: Vec<Predicate> = items[1..].iter()
                                .map(|i| self.desugar_predicate(i))
                                .collect::<Result<Vec<_>, _>>()?;
                            if preds.is_empty() { return Ok(Predicate::Lit(true)); }
                            let mut result = preds.remove(0);
                            for p in preds { result = Predicate::And(Box::new(result), Box::new(p)); }
                            Ok(result)
                        }
                        "or" | "||" => {
                            let mut preds: Vec<Predicate> = items[1..].iter()
                                .map(|i| self.desugar_predicate(i))
                                .collect::<Result<Vec<_>, _>>()?;
                            if preds.is_empty() { return Ok(Predicate::Lit(false)); }
                            let mut result = preds.remove(0);
                            for p in preds { result = Predicate::Or(Box::new(result), Box::new(p)); }
                            Ok(result)
                        }
                        "=" | "==" => {
                            if items.len() != 3 { return Err(DesugarError { message: "= needs 2 args".into(), span: expr.span }); }
                            Ok(Predicate::Cmp(CmpOp::Eq,
                                Box::new(self.desugar_term(&items[1])?),
                                Box::new(self.desugar_term(&items[2])?)))
                        }
                        "!=" => {
                            if items.len() != 3 { return Err(DesugarError { message: "!= needs 2 args".into(), span: expr.span }); }
                            Ok(Predicate::Cmp(CmpOp::Ne,
                                Box::new(self.desugar_term(&items[1])?),
                                Box::new(self.desugar_term(&items[2])?)))
                        }
                        "<" => {
                            if items.len() != 3 { return Err(DesugarError { message: "< needs 2 args".into(), span: expr.span }); }
                            Ok(Predicate::Cmp(CmpOp::Lt,
                                Box::new(self.desugar_term(&items[1])?),
                                Box::new(self.desugar_term(&items[2])?)))
                        }
                        "<=" => {
                            if items.len() != 3 { return Err(DesugarError { message: "<= needs 2 args".into(), span: expr.span }); }
                            Ok(Predicate::Cmp(CmpOp::Le,
                                Box::new(self.desugar_term(&items[1])?),
                                Box::new(self.desugar_term(&items[2])?)))
                        }
                        ">" => {
                            if items.len() != 3 { return Err(DesugarError { message: "> needs 2 args".into(), span: expr.span }); }
                            Ok(Predicate::Cmp(CmpOp::Gt,
                                Box::new(self.desugar_term(&items[1])?),
                                Box::new(self.desugar_term(&items[2])?)))
                        }
                        ">=" => {
                            if items.len() != 3 { return Err(DesugarError { message: ">= needs 2 args".into(), span: expr.span }); }
                            Ok(Predicate::Cmp(CmpOp::Ge,
                                Box::new(self.desugar_term(&items[1])?),
                                Box::new(self.desugar_term(&items[2])?)))
                        }
                        _ => {
                            let args: Vec<Predicate> = items[1..].iter()
                                .map(|i| self.desugar_predicate(i))
                                .collect::<Result<Vec<_>, _>>()?;
                            Ok(Predicate::App(op_name.clone(), args))
                        }
                    }
                } else {
                    Err(DesugarError { message: "predicate must start with sym".into(), span: expr.span })
                }
            }
            _ => Err(DesugarError { message: "invalid predicate".into(), span: expr.span }),
        }
    }

    fn desugar_term(&self, expr: &SExpr) -> Result<tisp_core::types::Term, DesugarError> {
        match &expr.node {
            Expr::Int(n) => Ok(tisp_core::types::Term::Lit(Lit::Int(*n))),
            Expr::Sym(name) => Ok(tisp_core::types::Term::Var(name.clone())),
            Expr::Bool(b) => Ok(tisp_core::types::Term::Lit(Lit::Bool(*b))),
            Expr::List(items) if !items.is_empty() => {
                if let Expr::Sym(op_name) = &items[0].node {
                    match op_name.as_str() {
                        "+" | "-" | "*" | "/" | "%" if items.len() == 3 => {
                            let op = match op_name.as_str() {
                                "+" => BinOp::Add, "-" => BinOp::Sub, "*" => BinOp::Mul,
                                "/" => BinOp::Div, "%" => BinOp::Mod, _ => unreachable!(),
                            };
                            Ok(tisp_core::types::Term::BinOp(op,
                                Box::new(self.desugar_term(&items[1])?),
                                Box::new(self.desugar_term(&items[2])?)))
                        }
                        _ => {
                            let args: Vec<tisp_core::types::Term> = items[1..].iter()
                                .map(|i| self.desugar_term(i))
                                .collect::<Result<Vec<_>, _>>()?;
                            Ok(tisp_core::types::Term::App(op_name.clone(), args))
                        }
                    }
                } else {
                    Err(DesugarError { message: "term must start with sym".into(), span: expr.span })
                }
            }
            _ => Err(DesugarError { message: "invalid term".into(), span: expr.span }),
        }
    }

    fn desugar_def_form(&self, items: &[SExpr], span: Span) -> Result<CoreDef, DesugarError> {
        if items.len() < 3 {
            return Err(DesugarError {
                message: "def requires name and body".into(),
                span,
            });
        }

        let name = match &items[1].node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError {
                message: "def name must be a symbol".into(),
                span: items[1].span,
            }),
        };

        let body = self.desugar_expr(&items[2])?;

        Ok(CoreDef {
            name,
            ty: None,
            effects: EffectRow::Pure,
            grade: Grade::Omega,
            mode: Mode::In,
            determinism: Determinism::Det,
            body,
            requires: None,
            ensures: None,
            span,
        })
    }

    fn desugar_defn_form(&self, items: &[SExpr], span: Span) -> Result<CoreDef, DesugarError> {
        if items.len() < 4 {
            return Err(DesugarError {
                message: "defn requires name, params, and body".into(),
                span,
            });
        }

        let name = match &items[1].node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError {
                message: "defn name must be a symbol".into(),
                span: items[1].span,
            }),
        };

        // ── Multi-arity detection ──
        // (defn name ([p1] b1) ([p2] b2) ...)
        if let Expr::List(first_clause) = &items[2].node {
            if !first_clause.is_empty() && matches!(&first_clause[0].node, Expr::Vec(_)) {
                return self.desugar_multi_arity_defn(name, &items[2..], span);
            }
        }

        let params = self.desugar_params(&items[2])?;

        // Look for -> ReturnType annotation
        let mut ret_type = None;
        let mut body_start = 3;
        while body_start < items.len() {
            if let Expr::Keyword(kw) = &items[body_start].node {
                if kw.as_str() == "->" {
                    if body_start + 1 < items.len() {
                        ret_type = Some(self.desugar_type_with_params(&items[body_start + 1], &[])?);
                        body_start += 2;
                    } else {
                        return Err(DesugarError { message: "-> requires a return type".into(), span: items[body_start].span });
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let mut requires = None;
        let mut ensures = None;

        let mut i = 3;
        let mut last_non_kw = None;
        while i < items.len() {
            if let Expr::Keyword(kw) = &items[i].node {
                match kw.as_str() {
                    ":requires" => {
                        if i + 1 < items.len() && !matches!(&items[i+1].node, Expr::Keyword(_)) {
                            requires = Some(self.desugar_predicate(&items[i + 1])?);
                            i += 2;
                        } else {
                            return Err(DesugarError { message: ":requires needs a predicate".into(), span: items[i].span });
                        }
                    }
                    ":ensures" => {
                        if i + 1 < items.len() && !matches!(&items[i+1].node, Expr::Keyword(_)) {
                            ensures = Some(self.desugar_predicate(&items[i + 1])?);
                            i += 2;
                        } else {
                            return Err(DesugarError { message: ":ensures needs a predicate".into(), span: items[i].span });
                        }
                    }
                    _ => {
                        last_non_kw = Some(i);
                        i += 1;
                    }
                }
            } else {
                last_non_kw = Some(i);
                i += 1;
            }
        }

        let _body_idx = last_non_kw.ok_or_else(|| DesugarError {
            message: "defn requires a body expression".into(),
            span,
        })?;

        // Collect all body expressions (non-keyword items after params/->)
        let mut body_exprs = Vec::new();
        for idx in body_start..items.len() {
            if let Expr::Keyword(_) = &items[idx].node { continue; }
            if idx > body_start {
                if let Expr::Keyword(kw) = &items[idx - 1].node {
                    if kw.as_str() == ":requires" || kw.as_str() == ":ensures" { continue; }
                }
            }
            body_exprs.push(self.desugar_expr(&items[idx])?);
        }

        let body = if body_exprs.len() == 1 {
            body_exprs.into_iter().next().unwrap()
        } else {
            CoreExpr::new(CoreExprNode::Do(body_exprs), span)
        };

        let lambda = CoreExprNode::Lam(Lambda {
            params,
            body: Box::new(body),
            ret_type: None,
        });

        Ok(CoreDef {
            name,
            ty: ret_type,
            effects: EffectRow::Pure,
            grade: Grade::Omega,
            mode: Mode::In,
            determinism: Determinism::Det,
            body: CoreExpr::new(lambda, span),
            requires,
            ensures,
            span,
        })
    }

    fn desugar_defeffect_form(&self, items: &[SExpr], span: Span) -> Result<tisp_core::effects::EffectDecl, DesugarError> {
        if items.len() < 3 {
            return Err(DesugarError { message: "defeffect requires name and at least one operation".into(), span });
        }
        // Parse name (possibly with type params): (defeffect State s ...) or (defeffect IO ...)
        let (name, type_params) = match &items[1].node {
            Expr::Sym(s) => (s.clone(), Vec::new()),
            Expr::List(name_items) if !name_items.is_empty() => {
                if let Expr::Sym(s) = &name_items[0].node {
                    let params = name_items[1..].iter().filter_map(|i| {
                        if let Expr::Sym(p) = &i.node { Some(p.clone()) } else { None }
                    }).collect();
                    (s.clone(), params)
                } else {
                    return Err(DesugarError { message: "effect name must be symbol".into(), span: items[1].span });
                }
            }
            _ => return Err(DesugarError { message: "effect name must be symbol or (name params)".into(), span: items[1].span }),
        };

        let mut operations = Vec::new();
        for item in &items[2..] {
            if let Expr::List(op_items) = &item.node {
                if op_items.len() >= 2 {
                    if let Expr::Sym(op_name) = &op_items[0].node {
                        let mut params = Vec::new();
                        let mut return_type = tisp_core::types::Type::unit();
                        // Parse operation: (get [] -> s) or (put [s] -> Unit)
                        for op_item in &op_items[1..] {
                            match &op_item.node {
                                Expr::Vec(v) => {
                                    for p in v {
                                        params.push(self.desugar_type_with_params(p, &type_params)?);
                                    }
                                }
                                Expr::Keyword(kw) if kw.as_str() == "->" => {}
                                _ => {
                                    // Might be a return type or type app
                                    return_type = self.desugar_type_with_params(op_item, &type_params)?;
                                }
                            }
                        }
                        operations.push(tisp_core::effects::OperationDecl {
                            name: op_name.clone(),
                            params,
                            return_type,
                        });
                    }
                }
            }
        }
        Ok(tisp_core::effects::EffectDecl { name, type_params, operations })
    }

    fn desugar_defpred_form(&self, items: &[SExpr], span: Span) -> Result<CoreDef, DesugarError> {
        if items.len() < 4 {
            return Err(DesugarError { message: "defpred requires name, params, and body".into(), span });
        }
        let name = match &items[1].node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError { message: "defpred name must be a symbol".into(), span: items[1].span }),
        };
        let params = self.desugar_params(&items[2])?;
        // Collect all body expressions (clauses) — defpred has multiple clauses
        let mut clauses = Vec::new();
        for idx in 3..items.len() {
            clauses.push(self.desugar_expr(&items[idx])?);
        }
        let body = if clauses.len() == 1 {
            clauses.into_iter().next().unwrap()
        } else {
            CoreExpr::new(CoreExprNode::Do(clauses), span)
        };
        let lambda = CoreExprNode::Lam(Lambda { params, body: Box::new(body), ret_type: None });
        Ok(CoreDef {
            name, ty: None, effects: EffectRow::Open(vec![EffectLabel::Search], Box::new(EffectRow::Pure)),
            grade: Grade::Omega, mode: Mode::Free, determinism: Determinism::NonDet,
            body: CoreExpr::new(lambda, span), requires: None, ensures: None, span,
        })
    }

    fn desugar_params(&self, expr: &SExpr) -> Result<Vec<Param>, DesugarError> {
        match &expr.node {
            Expr::Vec(items) => {
                let mut params = Vec::new();
                let mut i = 0;
                while i < items.len() {
                    let item = &items[i];
                    match &item.node {
                        Expr::Sym(name) => {
                            let grade = Grade::Omega;
                            let mut ty = None;
                            let name = name.clone();
                            // Check if next items are : type
                            if i + 2 < items.len() {
                                if let Expr::Keyword(kw) = &items[i + 1].node {
                                    if kw.as_str() == ":" {
                                        ty = Some(self.desugar_type_with_params(&items[i + 2], &[])?);
                                        i += 2; // skip : and type
                                    }
                                }
                            }
                            params.push(Param { name, ty, grade, mode: Mode::In });
                            i += 1;
                        }
                        // Graded parameter: (grade name : type) or (grade name)
                        Expr::List(parts) if !parts.is_empty() => {
                            self.desugar_graded_param(parts, &mut params)?;
                            i += 1;
                        }
                        // Graded parameter: {grade name : type} — parsed as Map
                        Expr::Map(pairs) if !pairs.is_empty() => {
                            let grade = match &pairs[0].0.node {
                                Expr::Int(0) => Grade::Zero,
                                Expr::Int(1) => Grade::One,
                                _ => Grade::Omega,
                            };
                            let name = match &pairs[0].1.node {
                                Expr::Sym(s) => s.clone(),
                                _ => return Err(DesugarError { message: "graded param name must be symbol".into(), span: item.span }),
                            };
                            let ty = if pairs.len() >= 2 {
                                match &pairs[1].0.node {
                                    Expr::Keyword(k) if k.as_str() == ":" => Some(self.desugar_type_with_params(&pairs[1].1, &[])?),
                                    _ => None,
                                }
                            } else { None };
                            params.push(Param { name, ty, grade, mode: Mode::In });
                            i += 1;
                        }
                        // Graded parameter: {grade name : type} — parsed as Set
                        Expr::Set(items) if !items.is_empty() => {
                            match &items[0].node {
                                Expr::Int(0) | Expr::Int(1) | Expr::Sym(_) => {
                                    if items.len() < 2 { return Err(DesugarError { message: "graded param needs a name".into(), span: item.span }); }
                                    let grade = match &items[0].node {
                                        Expr::Int(0) => Grade::Zero,
                                        Expr::Int(1) => Grade::One,
                                        _ => Grade::Omega,
                                    };
                                    let name = match &items[1].node {
                                        Expr::Sym(s) => s.clone(),
                                        _ => return Err(DesugarError { message: "graded param needs a name symbol".into(), span: items[1].span }),
                                    };
                                    let ty = if items.len() >= 4 && matches!(&items[2].node, Expr::Keyword(k) if k.as_str() == ":") {
                                        Some(self.desugar_type_with_params(&items[3], &[])?)
                                    } else { None };
                                    params.push(Param { name, ty, grade, mode: Mode::In });
                                }
                                _ => return Err(DesugarError { message: "graded param grade must be 0, 1, or ω".into(), span: items[0].span }),
                            }
                            i += 1;
                        }
                        // Skip standalone separator tokens
                        Expr::Keyword(_) => { i += 1; }
                        _ => return Err(DesugarError {
                            message: "parameter must be a symbol or (grade name : type)".into(),
                            span: item.span,
                        }),
                    }
                }
                Ok(params)
            }
            _ => Err(DesugarError {
                message: "parameters must be a vector".into(),
                span: expr.span,
            }),
        }
    }

    fn desugar_graded_param(&self, parts: &[SExpr], params: &mut Vec<Param>) -> Result<(), DesugarError> {
        if parts.is_empty() { return Ok(()); }
        // Parse grade from first element
        let grade = match &parts[0].node {
            Expr::Int(0) => Grade::Zero,
            Expr::Int(1) => Grade::One,
            Expr::Sym(s) if s.as_str() == "ω" || s.as_str() == "omega" => Grade::Omega,
            _ => return Err(DesugarError {
                message: "grade must be 0, 1, or ω".into(),
                span: parts[0].span,
            }),
        };
        // Parse name from second element
        if parts.len() < 2 {
            return Err(DesugarError { message: "graded param needs a name".into(), span: parts[0].span });
        }
        let name = match &parts[1].node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError { message: "parameter name must be a symbol".into(), span: parts[1].span }),
        };
        // Parse optional type annotation: : type
        let ty = if parts.len() >= 3 {
            // Check for : separator
            if let Expr::Keyword(kw) = &parts[2].node {
                if kw.as_str() == ":" && parts.len() >= 4 {
                    Some(self.desugar_type_with_params(&parts[3], &[])?)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        params.push(Param { name, ty, grade, mode: Mode::In });
        Ok(())
    }

    pub fn desugar_expr(&self, expr: &SExpr) -> Result<CoreExpr, DesugarError> {
        match &expr.node {
            Expr::Nil => Ok(CoreExpr::new(
                CoreExprNode::Lit(Literal::Unit),
                expr.span,
            )),
            Expr::Bool(b) => Ok(CoreExpr::new(
                CoreExprNode::Lit(Literal::Bool(*b)),
                expr.span,
            )),
            Expr::Int(n) => Ok(CoreExpr::new(
                CoreExprNode::Lit(Literal::I64(*n)),
                expr.span,
            )),
            Expr::Float(f) => Ok(CoreExpr::new(
                CoreExprNode::Lit(Literal::F64(*f)),
                expr.span,
            )),
            Expr::Str(s) => Ok(CoreExpr::new(
                CoreExprNode::Lit(Literal::String(s.clone())),
                expr.span,
            )),
            Expr::Char(c) => Ok(CoreExpr::new(
                CoreExprNode::Lit(Literal::Char(*c)),
                expr.span,
            )),
            Expr::Sym(name) => {
                if name.as_str() == "i0" {
                    Ok(CoreExpr::new(CoreExprNode::IntervalEndpoint(false), expr.span))
                } else if name.as_str() == "i1" {
                    Ok(CoreExpr::new(CoreExprNode::IntervalEndpoint(true), expr.span))
                } else if name.as_str().starts_with('?') {
                    let hole_name = Symbol::new(&name.as_str()[1..]);
                    Ok(CoreExpr::new(
                        CoreExprNode::Hole(Some(hole_name)),
                        expr.span,
                    ))
                } else {
                    Ok(CoreExpr::new(
                        CoreExprNode::Var(name.clone()),
                        expr.span,
                    ))
                }
            }
            Expr::Keyword(kw) => {
                // Keywords are self-evaluating symbols
                Ok(CoreExpr::new(
                    CoreExprNode::Lit(Literal::String(format!(":{}", kw))),
                    expr.span,
                ))
            }
            Expr::List(items) => self.desugar_list(items, expr.span),
            Expr::Vec(items) => {
                // Vector literal - desugar to Vec constructor
                let mut args = Vec::new();
                for item in items {
                    args.push(self.desugar_expr(item)?);
                }
                Ok(CoreExpr::new(
                    CoreExprNode::Data(Symbol::new("Vec"), args),
                    expr.span,
                ))
            }
            Expr::ConsPattern(items, tail) => {
                let mut args: Vec<CoreExpr> = items.iter().map(|i| self.desugar_expr(i)).collect::<Result<_, _>>()?;
                args.push(self.desugar_expr(tail)?);
                Ok(CoreExpr::new(
                    CoreExprNode::Data(Symbol::new("Cons"), args),
                    expr.span,
                ))
            }
            Expr::Map(pairs) => {
                // Map literal - desugar to Map constructor
                let mut args = Vec::new();
                for (k, v) in pairs {
                    let pair = CoreExpr::new(
                        CoreExprNode::Data(
                            Symbol::new("Pair"),
                            vec![self.desugar_expr(k)?, self.desugar_expr(v)?],
                        ),
                        k.span.merge(v.span),
                    );
                    args.push(pair);
                }
                Ok(CoreExpr::new(
                    CoreExprNode::Data(Symbol::new("Map"), args),
                    expr.span,
                ))
            }
            Expr::Set(items) => {
                // Set literal - desugar to Set constructor
                let mut args = Vec::new();
                for item in items {
                    args.push(self.desugar_expr(item)?);
                }
                Ok(CoreExpr::new(
                    CoreExprNode::Data(Symbol::new("Set"), args),
                    expr.span,
                ))
            }
            Expr::Quote(inner) => {
                // Quote - for now, just desugar the inner expression
                // TODO: proper quote handling
                self.desugar_expr(inner)
            }
            Expr::SyntaxQuote(inner) => {
                // syntax-quote: construct a list literal with the inner expression as-is
                Ok(CoreExpr::new(CoreExprNode::Data(Symbol::new("Quote"), vec![self.desugar_expr(inner)?]), expr.span))
            },
            Expr::Unquote(inner) => {
                // unquote ~x: evaluate x and substitute
                self.desugar_expr(inner)
            },
            Expr::UnquoteSplice(inner) => {
                // unquote-splice ~@items: evaluate items and splice into enclosing list
                self.desugar_expr(inner)
            },
        }
    }

    fn desugar_list(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.is_empty() {
            return Ok(CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span));
        }

        match &items[0].node {
            Expr::Keyword(kw) => {
                match kw.as_str() {
                    "->" => self.desugar_thread_first(items, span),
                    "->>" => self.desugar_thread_last(items, span),
                    _ => self.desugar_app(items, span),
                }
            }
            Expr::Sym(name) => {
                match name.as_str() {
                    "fn" => self.desugar_lambda(items, span),
                    "let" => self.desugar_let(items, span),
                    "if" => self.desugar_if(items, span),
                    "cond" => self.desugar_cond(items, span),
                    "if-let" => self.desugar_if_let(items, span),
                    "when-let" => self.desugar_when_let(items, span),
                    "and" => self.desugar_and(items, span),
                    "or" => self.desugar_or(items, span),
                    "->>" => self.desugar_thread_last(items, span),
                    "as->" => self.desugar_as_thread(items, span),
                    "some->" => self.desugar_some_thread(items, span),
                    "match" => self.desugar_match(items, span),
                    "handle" => self.desugar_handle(items, span),
                    "perform" => self.desugar_perform(items, span),
                    "do" => self.desugar_do(items, span),
                    // Logic
                    "fresh"     => self.desugar_new_fresh(items, span),
                    "=="        => self.desugar_binary_wrap(|a, b| CoreExprNode::Unify(a, b), items, span),
                    "unify"     => self.desugar_binary_wrap(|a, b| CoreExprNode::Unify(a, b), items, span),
                    "search"    => self.desugar_unary_wrap(|e| CoreExprNode::Search(e), items, span),
                    "commit"    => self.desugar_unary_wrap(|e| CoreExprNode::Commit(e), items, span),
                    "abduce"    => self.desugar_abduce(items, span),
                    // Constraint
                    "constrain" => self.desugar_unary_wrap(|e| CoreExprNode::Constrain(e), items, span),
                    "label"     => self.desugar_binary_wrap(|a, b| CoreExprNode::Label(a, b), items, span),
                    "all-diff"  => self.desugar_list_wrap(|xs| CoreExprNode::AllDifferent(xs), items, span),
                    "domain"    => self.desugar_ternary(|a, b, c| CoreExprNode::Domain(a, b, c), items, span),
                    // Process
                    "spawn"     => self.desugar_spawn(items, span),
                    "join"      => self.desugar_unary_wrap(|e| CoreExprNode::Join(e), items, span),
                    "chan"      => self.desugar_v0(CoreExprNode::ChannelNew, items, span),
                    "send!"     => self.desugar_binary_wrap(|a, b| CoreExprNode::ChannelSend(a, b), items, span),
                    "recv!"     => self.desugar_unary_wrap(|e| CoreExprNode::ChannelRecv(e), items, span),
                    "async-send" => self.desugar_binary_wrap(|a, b| CoreExprNode::AsyncSend(a, b), items, span),
                    "async-recv" => self.desugar_unary_wrap(|e| CoreExprNode::AsyncRecv(e), items, span),
                    "enter"     => self.desugar_binary_wrap(|a, b| CoreExprNode::AmbientEnter(a, b), items, span),
                    "exit"      => self.desugar_binary_wrap(|a, b| CoreExprNode::AmbientExit(a, b), items, span),
                    "open"      => self.desugar_binary_wrap(|a, b| CoreExprNode::AmbientOpen(a, b), items, span),
                    "rho-quote" => self.desugar_unary_wrap(|e| CoreExprNode::RhoQuote(e), items, span),
                    "rho-drop"  => self.desugar_unary_wrap(|e| CoreExprNode::RhoDrop(e), items, span),
                    "rho-lift"  => self.desugar_binary_wrap(|a, b| CoreExprNode::RhoLift(a, b), items, span),
                    // Applied π-calculus
                    "encrypt"   => self.desugar_binary_wrap(|a, b| CoreExprNode::CryptoEncrypt(a, b), items, span),
                    "decrypt"   => self.desugar_binary_wrap(|a, b| CoreExprNode::CryptoDecrypt(a, b), items, span),
                    "sign"      => self.desugar_binary_wrap(|a, b| CoreExprNode::CryptoSign(a, b), items, span),
                    "verify"    => self.desugar_binary_wrap(|a, b| CoreExprNode::CryptoVerify(a, b), items, span),
                    "hash"      => self.desugar_unary_wrap(|e| CoreExprNode::CryptoHash(e), items, span),
                    // spi-calculus
                    "secret!"   => self.desugar_unary_wrap(|e| CoreExprNode::SpiSecret(e), items, span),
                    "commit!"   => self.desugar_binary_wrap(|a, b| CoreExprNode::SpiCommit(a, b), items, span),
                    "check!"    => self.desugar_binary_wrap(|a, b| CoreExprNode::SpiCheck(a, b), items, span),
                    // SKI
                    "S"         => self.desugar_v0(CoreExprNode::SkiS, items, span),
                    "K"         => self.desugar_v0(CoreExprNode::SkiK, items, span),
                    "I"         => self.desugar_v0(CoreExprNode::SkiI, items, span),
                    "ski-app"   => self.desugar_binary_wrap(|a, b| CoreExprNode::SkiApp(a, b), items, span),
                    "ski-reduce" => self.desugar_unary_wrap(|e| CoreExprNode::SkiReduce(e), items, span),
                    // ς-calculus
                    "invoke"    => self.desugar_binary_wrap(|a, b| CoreExprNode::SigmaInvoke(a, b), items, span),
                    "update!"   => self.desugar_binary_wrap(|a, b| CoreExprNode::SigmaUpdate(a, b), items, span),
                    // HoTT
                    "flat"      => self.desugar_hott_unary(CoreExprNode::FlatMod, items, span),
                    "sharp"     => self.desugar_hott_unary(CoreExprNode::SharpMod, items, span),
                    "path-lam"  => self.desugar_path_lam(items, span),
                    "path-apply" => self.desugar_path_apply(items, span),
                    "glue"      => self.desugar_binary_wrap(|a, b| CoreExprNode::Glue(a, b), items, span),
                    "unglue"    => self.desugar_unary_wrap(|e| CoreExprNode::Unglue(e), items, span),
                    // FRP
                    "signal"    => self.desugar_unary_wrap(|e| CoreExprNode::SignalNew(e), items, span),
                    "signal-map" => self.desugar_binary_wrap(|a, b| CoreExprNode::SignalMap(a, b), items, span),
                    "signal-filter" => self.desugar_binary_wrap(|a, b| CoreExprNode::SignalFilter(a, b), items, span),
                    "signal-fold" => self.desugar_ternary(|a, b, c| CoreExprNode::SignalFold(a, b, c), items, span),
                    "delay"     => self.desugar_unary_wrap(|e| CoreExprNode::Delay(e), items, span),
                    "advance"   => self.desugar_unary_wrap(|e| CoreExprNode::Advance(e), items, span),
                    "stable"    => self.desugar_unary_wrap(|e| CoreExprNode::Stable(e), items, span),
                    "unbox!"    => self.desugar_unary_wrap(|e| CoreExprNode::Unbox(e), items, span),
                    // Metaprogramming
                    "comptime"  => self.desugar_unary_wrap(|e| CoreExprNode::Comptime(e), items, span),
                    // Memory
                    "region-alloc" => self.desugar_binary_wrap(|a, b| CoreExprNode::RegionAlloc(a, b), items, span),
                    "region-free" => self.desugar_unary_wrap(|e| CoreExprNode::RegionFree(e), items, span),
                    "ptr-read"  => self.desugar_unary_wrap(|e| CoreExprNode::PtrRead(e), items, span),
                    "ptr-write" => self.desugar_binary_wrap(|a, b| CoreExprNode::PtrWrite(a, b), items, span),
                    // Session
                    "send"      => self.desugar_session(SessionOp::Send, items, span),
                    "recv"      => self.desugar_session(SessionOp::Recv, items, span),
                    "close"     => self.desugar_session(SessionOp::Close, items, span),
                    _ => self.desugar_app(items, span),
                }
            }
            _ => self.desugar_app(items, span),
        }
    }

    fn desugar_lambda(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 3 {
            return Err(DesugarError {
                message: "fn requires params and body".into(),
                span,
            });
        }

        let params = self.desugar_params(&items[1])?;
        let body = self.desugar_expr(&items[2])?;

        Ok(CoreExpr::new(
            CoreExprNode::Lam(Lambda {
                params,
                body: Box::new(body),
                ret_type: None,
            }),
            span,
        ))
    }

    fn desugar_let(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 3 {
            return Err(DesugarError {
                message: "let requires bindings and body".into(),
                span,
            });
        }

        let bindings = match &items[1].node {
            Expr::Vec(binds) => binds,
            _ => return Err(DesugarError {
                message: "let bindings must be a vector".into(),
                span: items[1].span,
            }),
        };

        if bindings.len() % 2 != 0 {
            return Err(DesugarError {
                message: "let bindings must have even number of elements".into(),
                span: items[1].span,
            });
        }

        let body = self.desugar_expr(&items[2])?;

        // Build nested lets from right to left
        let mut result = body;
        for i in (0..bindings.len()).step_by(2).rev() {
            let name = match &bindings[i].node {
                Expr::Sym(s) => s.clone(),
                _ => return Err(DesugarError {
                    message: "let binding name must be a symbol".into(),
                    span: bindings[i].span,
                }),
            };
            let value = self.desugar_expr(&bindings[i + 1])?;

            result = CoreExpr::new(
                CoreExprNode::Let(name, None, Box::new(value), Box::new(result)),
                span,
            );
        }

        Ok(result)
    }

    fn desugar_if(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() != 4 {
            return Err(DesugarError {
                message: "if requires condition, then, and else".into(),
                span,
            });
        }

        let cond = self.desugar_expr(&items[1])?;
        let then = self.desugar_expr(&items[2])?;
        let else_ = self.desugar_expr(&items[3])?;

        Ok(CoreExpr::new(
            CoreExprNode::If(Box::new(cond), Box::new(then), Box::new(else_)),
            span,
        ))
    }

    fn desugar_cond(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 2 {
            return Err(DesugarError { message: "cond requires at least one clause".into(), span });
        }
        let clauses = &items[1..];
        let mut result = self.desugar_expr(&items[items.len() - 1])?;
        let mut i = clauses.len();
        while i >= 2 {
            let body = &clauses[i - 1];
            let test = &clauses[i - 2];
            i -= 2;
            if let Expr::Keyword(kw) = &test.node {
                if kw.as_str() == "else" {
                    result = self.desugar_expr(body)?;
                    continue;
                }
            }
            result = CoreExpr::new(
                CoreExprNode::If(
                    Box::new(self.desugar_expr(test)?),
                    Box::new(self.desugar_expr(body)?),
                    Box::new(result),
                ),
                span,
            );
        }
        Ok(result)
    }

    fn desugar_if_let(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 4 {
            return Err(DesugarError { message: "if-let requires [var expr] then else".into(), span });
        }
        let (var, val_expr) = self.desugar_let_binding(&items[1])?;
        let then_body = self.desugar_expr(&items[2])?;
        let else_body = if items.len() > 3 { self.desugar_expr(&items[3])? }
            else { CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span) };
        let var_sym = match &var.node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError { message: "if-let requires a variable name".into(), span: var.span }),
        };
        let test = CoreExpr::new(CoreExprNode::Var(var_sym.clone()), span);
        Ok(CoreExpr::new(
            CoreExprNode::Let(var_sym, None, Box::new(val_expr),
                Box::new(CoreExpr::new(
                    CoreExprNode::If(Box::new(test), Box::new(then_body), Box::new(else_body)),
                    span,
                )),
            ),
            span,
        ))
    }

    fn desugar_when_let(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 3 {
            return Err(DesugarError { message: "when-let requires [var expr] body".into(), span });
        }
        let (var, val_expr) = self.desugar_let_binding(&items[1])?;
        let body = self.desugar_expr(&items[2])?;
        let else_ = CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span);
        let var_sym = match &var.node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError { message: "when-let requires a variable name".into(), span: var.span }),
        };
        let test = CoreExpr::new(CoreExprNode::Var(var_sym.clone()), span);
        Ok(CoreExpr::new(
            CoreExprNode::Let(var_sym, None, Box::new(val_expr),
                Box::new(CoreExpr::new(
                    CoreExprNode::If(Box::new(test), Box::new(body), Box::new(else_)),
                    span,
                )),
            ),
            span,
        ))
    }

    fn desugar_let_binding(&self, expr: &SExpr) -> Result<(SExpr, CoreExpr), DesugarError> {
        match &expr.node {
            Expr::Vec(items) if items.len() == 2 => {
                Ok((items[0].clone(), self.desugar_expr(&items[1])?))
            }
            _ => Err(DesugarError { message: "expected [var expr] binding".into(), span: expr.span }),
        }
    }

    fn desugar_multi_arity_defn(&self, name: Symbol, clauses: &[SExpr], span: Span) -> Result<CoreDef, DesugarError> {
        let mut arms = Vec::new();
        let mut all_params = Vec::new();
        for clause in clauses {
            if let Expr::List(inner) = &clause.node {
                if inner.len() >= 2 {
                    if let Expr::Vec(param_items) = &inner[0].node {
                        let mut pat_vars = Vec::new();
                        for p in param_items {
                            if let Expr::Sym(s) = &p.node {
                                pat_vars.push(Pattern::Var(s.clone()));
                                if !all_params.contains(s) { all_params.push(s.clone()); }
                            } else {
                                pat_vars.push(Pattern::Wildcard);
                            }
                        }
                        let body = self.desugar_expr(&inner[1])?;
                        arms.push(MatchArm {
                            pattern: if pat_vars.len() == 1 { pat_vars[0].clone() } else { Pattern::Tuple(pat_vars) },
                            guard: None,
                            body: Box::new(body),
                        });
                    }
                }
            }
        }
        if arms.is_empty() {
            return Err(DesugarError { message: "defn multi-arity requires at least one valid clause".into(), span });
        }
        let params: Vec<Param> = all_params.iter().map(|s| Param {
            name: s.clone(), ty: None, grade: Grade::Omega, mode: Mode::In,
        }).collect();
        let match_body = CoreExpr::new(CoreExprNode::Match(
            Box::new(CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span)),
            arms,
        ), span);
        let lambda = CoreExprNode::Lam(Lambda { params, body: Box::new(match_body), ret_type: None });
        Ok(CoreDef {
            name,
            ty: None, effects: EffectRow::Pure, grade: Grade::Omega, mode: Mode::In,
            determinism: Determinism::Det, body: CoreExpr::new(lambda, span),
            requires: None, ensures: None, span,
        })
    }

    fn desugar_and(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        let exprs: Vec<_> = items[1..].iter().map(|e| self.desugar_expr(e)).collect::<Result<_, _>>()?;
        let falsy = CoreExpr::new(CoreExprNode::Lit(Literal::Bool(false)), span);
        let result = exprs.into_iter().rfold(falsy, |acc, e| {
            CoreExpr::new(CoreExprNode::If(Box::new(e.clone()), Box::new(acc), Box::new(e)), span)
        });
        Ok(result)
    }

    fn desugar_thread_first(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 2 { return Err(DesugarError { message: "-> requires at least a value".into(), span }); }
        let mut result = self.desugar_expr(&items[1])?;
        for item in &items[2..] {
            result = match &item.node {
                Expr::Sym(f) => CoreExpr::new(
                    CoreExprNode::App(Box::new(CoreExpr::new(CoreExprNode::Var(f.clone()), item.span)), Box::new(result)),
                    span,
                ),
                Expr::List(parts) if !parts.is_empty() => {
                    let mut args = vec![result];
                    for p in &parts[1..] { args.push(self.desugar_expr(p)?); }
                    let func = self.desugar_expr(&parts[0])?;
                    let mut app = func;
                    for arg in args {
                        app = CoreExpr::new(CoreExprNode::App(Box::new(app), Box::new(arg)), span);
                    }
                    app
                }
                _ => continue,
            };
        }
        Ok(result)
    }

    fn desugar_thread_last(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 2 { return Err(DesugarError { message: "->> requires at least a value".into(), span }); }
        let mut result = self.desugar_expr(&items[1])?;
        for item in &items[2..] {
            result = match &item.node {
                Expr::Sym(f) => CoreExpr::new(
                    CoreExprNode::App(Box::new(CoreExpr::new(CoreExprNode::Var(f.clone()), item.span)), Box::new(result)),
                    span,
                ),
                Expr::List(parts) if !parts.is_empty() => {
                    let mut args = Vec::new();
                    for p in &parts[1..] { args.push(self.desugar_expr(p)?); }
                    args.push(result);
                    let func = self.desugar_expr(&parts[0])?;
                    let mut app = func;
                    for arg in args {
                        app = CoreExpr::new(CoreExprNode::App(Box::new(app), Box::new(arg)), span);
                    }
                    app
                }
                _ => continue,
            };
        }
        Ok(result)
    }

    fn desugar_as_thread(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 3 { return Err(DesugarError { message: "as-> requires expr, name, and forms".into(), span }); }
        let init = self.desugar_expr(&items[1])?;
        let name = match &items[2].node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError { message: "as-> requires a binding name".into(), span: items[2].span }),
        };
        let mut result = init;
        for item in &items[3..] {
            let step = match &item.node {
                Expr::List(parts) if !parts.is_empty() => {
                    let mut args: Vec<CoreExpr> = vec![CoreExpr::new(CoreExprNode::Var(name.clone()), span)];
                    for p in &parts[1..] { args.push(self.desugar_expr(p)?); }
                    let func = self.desugar_expr(&parts[0])?;
                    args.insert(0, func);
                    let mut app = args.remove(0);
                    for arg in args {
                        app = CoreExpr::new(CoreExprNode::App(Box::new(app), Box::new(arg)), span);
                    }
                    app
                }
                _ => {
                    let bind = CoreExpr::new(CoreExprNode::Let(name.clone(), None, Box::new(result), Box::new(self.desugar_expr(item)?)), span);
                    return Ok(bind);
                }
            };
            result = CoreExpr::new(CoreExprNode::Let(name.clone(), None, Box::new(result), Box::new(step)), span);
        }
        Ok(result)
    }

    fn desugar_some_thread(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 2 { return Err(DesugarError { message: "some-> requires at least a value".into(), span }); }
        let nil_check = CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span);
        let mut result = self.desugar_expr(&items[1])?;
        for item in &items[2..] {
            result = match &item.node {
                Expr::Sym(f) => CoreExpr::new(
                    CoreExprNode::If(
                        Box::new(CoreExpr::new(
                            CoreExprNode::App(
                                Box::new(CoreExpr::new(CoreExprNode::Var(Symbol::new("=")), item.span)),
                                Box::new(result.clone()),
                            ), span,
                        )),
                        Box::new(nil_check.clone()),
                        Box::new(CoreExpr::new(
                            CoreExprNode::App(Box::new(CoreExpr::new(CoreExprNode::Var(f.clone()), item.span)), Box::new(result)),
                            span,
                        )),
                    ),
                    span,
                ),
                Expr::List(parts) if !parts.is_empty() => {
                    let mut args = vec![result];
                    for p in &parts[1..] { args.push(self.desugar_expr(p)?); }
                    let func = self.desugar_expr(&parts[0])?;
                    let mut app = func;
                    for arg in args {
                        app = CoreExpr::new(CoreExprNode::App(Box::new(app), Box::new(arg)), span);
                    }
                    app
                }
                _ => continue,
            };
        }
        Ok(result)
    }

    fn desugar_or(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        let exprs: Vec<_> = items[1..].iter().map(|e| self.desugar_expr(e)).collect::<Result<_, _>>()?;
        let falsy = CoreExpr::new(CoreExprNode::Lit(Literal::Bool(false)), span);
        if exprs.is_empty() { return Ok(falsy); }
        let mut result = exprs.last().unwrap().clone();
        for e in exprs.iter().rev().skip(1) {
            result = CoreExpr::new(CoreExprNode::If(Box::new(e.clone()), Box::new(e.clone()), Box::new(result)), span);
        }
        Ok(result)
    }

    fn desugar_match(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 3 {
            return Err(DesugarError {
                message: "match requires scrutinee and at least one pattern/body pair".into(),
                span,
            });
        }

        let scrutinee = self.desugar_expr(&items[1])?;
        let mut arms = Vec::new();

        let mut i = 2;
        while i < items.len() {
            let pattern = self.desugar_pattern(&items[i])?;
            i += 1;
            if i >= items.len() {
                return Err(DesugarError { message: "match arm missing body after pattern".into(), span });
            }
            let guard = if i + 1 < items.len() &&
                matches!(&items[i].node, Expr::Keyword(k) if k.as_str() == "when") {
                let guard_expr = self.desugar_expr(&items[i + 1])?;
                i += 2;
                if i >= items.len() {
                    return Err(DesugarError { message: "match arm missing body after :when guard".into(), span });
                }
                Some(Box::new(guard_expr))
            } else { None };
            let body = self.desugar_expr(&items[i])?;
            i += 1;
            arms.push(MatchArm {
                pattern,
                guard,
                body: Box::new(body),
            });
        }

        Ok(CoreExpr::new(
            CoreExprNode::Match(Box::new(scrutinee), arms),
            span,
        ))
    }

    fn desugar_pattern(&self, expr: &SExpr) -> Result<Pattern, DesugarError> {
        match &expr.node {
            Expr::Sym(name) if name.as_str() == "_" => Ok(Pattern::Wildcard),
            Expr::Sym(name) => Ok(Pattern::Var(name.clone())),
            Expr::Int(n) => Ok(Pattern::Lit(Literal::I64(*n))),
            Expr::Bool(b) => Ok(Pattern::Lit(Literal::Bool(*b))),
            Expr::Str(s) => Ok(Pattern::Lit(Literal::String(s.clone()))),
            Expr::List(items) if !items.is_empty() => {
                if let Expr::Sym(con_name) = &items[0].node {
                    if con_name.as_str() == "or" {
                        // (or pat1 pat2 ...) — for now, take the first pattern
                        if items.len() > 1 { return self.desugar_pattern(&items[1]); }
                        return Err(DesugarError { message: "or-pattern needs at least one subpattern".into(), span: expr.span });
                    }
                    let mut subpats = Vec::new();
                    for item in &items[1..] {
                        subpats.push(self.desugar_pattern(item)?);
                    }
                    Ok(Pattern::Con(con_name.clone(), subpats))
                } else {
                    Err(DesugarError { message: "pattern constructor must be a symbol".into(), span: items[0].span })
                }
            }
            Expr::ConsPattern(items, tail) => {
                let tail_pat = self.desugar_pattern(tail)?;
                let mut result = tail_pat;
                for item in items.iter().rev() {
                    let head = self.desugar_pattern(item)?;
                    result = Pattern::Con(
                        Symbol::new("Cons"),
                        vec![head, result],
                    );
                }
                Ok(result)
            }
            _ => Err(DesugarError { message: "invalid pattern".into(), span: expr.span }),
        }
    }

    fn desugar_do(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 2 {
            return Ok(CoreExpr::new(CoreExprNode::Lit(Literal::Unit), span));
        }
        let exprs: Vec<CoreExpr> = items[1..].iter()
            .map(|item| self.desugar_expr(item))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(CoreExpr::new(CoreExprNode::Do(exprs), span))
    }

    fn desugar_spawn(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 2 {
            return Err(DesugarError { message: "spawn requires a body expression".into(), span });
        }
        let body = self.desugar_expr(&items[1])?;
        // Generate a handle name for structured concurrency
        let handle_name = Symbol::new("_join_handle");
        Ok(CoreExpr::new(CoreExprNode::Spawn(Box::new(body), Box::new(CoreExpr::new(CoreExprNode::Var(handle_name), span))), span))
    }

    // ── Generic desugar helpers for N-ary primitives ──

    fn desugar_unary_wrap(&self, wrap: fn(Box<CoreExpr>) -> CoreExprNode, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() != 2 { return Err(DesugarError { message: "expects 1 argument".into(), span }); }
        Ok(CoreExpr::new(wrap(Box::new(self.desugar_expr(&items[1])?)), span))
    }

    fn desugar_binary_wrap(&self, wrap: fn(Box<CoreExpr>, Box<CoreExpr>) -> CoreExprNode, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() != 3 { return Err(DesugarError { message: "expects 2 arguments".into(), span }); }
        Ok(CoreExpr::new(wrap(Box::new(self.desugar_expr(&items[1])?), Box::new(self.desugar_expr(&items[2])?)), span))
    }

    fn desugar_ternary(&self, wrap: fn(Box<CoreExpr>, Box<CoreExpr>, Box<CoreExpr>) -> CoreExprNode, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() != 4 { return Err(DesugarError { message: "expects 3 arguments".into(), span }); }
        Ok(CoreExpr::new(wrap(Box::new(self.desugar_expr(&items[1])?), Box::new(self.desugar_expr(&items[2])?), Box::new(self.desugar_expr(&items[3])?)), span))
    }

    fn desugar_list_wrap(&self, wrap: fn(Vec<CoreExpr>) -> CoreExprNode, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        let exprs: Vec<CoreExpr> = items[1..].iter().map(|i| self.desugar_expr(i)).collect::<Result<Vec<_>, _>>()?;
        Ok(CoreExpr::new(wrap(exprs), span))
    }

    fn desugar_v0(&self, wrap: CoreExprNode, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() != 1 { return Err(DesugarError { message: "expects no arguments".into(), span }); }
        Ok(CoreExpr::new(wrap, span))
    }

    fn desugar_new_fresh(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() != 2 { return Err(DesugarError { message: "fresh requires a variable name".into(), span }); }
        let name = match &items[1].node { Expr::Sym(s) => s.clone(), _ => return Err(DesugarError { message: "expected symbol".into(), span: items[1].span }) };
        Ok(CoreExpr::new(CoreExprNode::Fresh(name), span))
    }

    fn desugar_abduce(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 3 { return Err(DesugarError { message: "abduce requires goal and abducibles".into(), span }); }
        let goal = self.desugar_expr(&items[1])?;
        let mut abducibles = Vec::new();
        for item in &items[2..] {
            if let Expr::Sym(s) = &item.node { abducibles.push(s.clone()); }
            else { return Err(DesugarError { message: "abducibles must be symbols".into(), span: item.span }); }
        }
        Ok(CoreExpr::new(CoreExprNode::Abduce(Box::new(goal), abducibles), span))
    }

    fn desugar_hott_unary(&self, wrap: fn(Box<CoreExpr>) -> CoreExprNode, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() != 2 {
            return Err(DesugarError { message: "flat/sharp expects 1 argument".into(), span });
        }
        Ok(CoreExpr::new(wrap(Box::new(self.desugar_expr(&items[1])?)), span))
    }

    fn desugar_path_lam(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() != 3 {
            return Err(DesugarError { message: "path-lam requires var and body".into(), span });
        }
        let var = match &items[1].node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError { message: "path-lam var must be symbol".into(), span }),
        };
        Ok(CoreExpr::new(CoreExprNode::PathLam(var, Box::new(self.desugar_expr(&items[2])?)), span))
    }

    fn desugar_path_apply(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() != 3 {
            return Err(DesugarError { message: "path-apply requires path and point".into(), span });
        }
        Ok(CoreExpr::new(CoreExprNode::PathApp(
            Box::new(self.desugar_expr(&items[1])?),
            Box::new(self.desugar_expr(&items[2])?),
        ), span))
    }

    fn desugar_session(&self, op: SessionOp, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 2 {
            return Err(DesugarError { message: "session op requires operand".into(), span });
        }
        Ok(CoreExpr::new(CoreExprNode::Session(op, Box::new(self.desugar_expr(&items[1])?)), span))
    }

    fn desugar_handle(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 3 {
            return Err(DesugarError { message: "handle requires body and handler".into(), span });
        }
        let body = self.desugar_expr(&items[1])?;

        let mut effect_name = Symbol::new("Unknown");
        let mut type_args = Vec::new();
        let mut clauses = Vec::new();
        let mut i = 2;
        while i < items.len() {
            if let Expr::List(inner) = &items[i].node {
                if inner.is_empty() { i += 1; continue; }
                if let Expr::Sym(first) = &inner[0].node {
                    // Check if this is an operation clause: (op-name [params] [continuation-vars] body)
                    if inner.len() >= 4 {
                        if let (Expr::Sym(_), Expr::Vec(params), Expr::Vec(cont_vars)) =
                            (&inner[0].node, &inner[1].node, &inner[2].node) {
                            let mut op_params = Vec::new();
                            for p in params {
                                if let Expr::Sym(s) = &p.node { op_params.push(s.clone()); }
                                else { op_params.push(Symbol::new("_")); }
                            }
                            let mut cont = Vec::new();
                            for c in cont_vars {
                                if let Expr::Sym(s) = &c.node { cont.push(s.clone()); }
                            }
                            let cont_name = cont.first().cloned().unwrap_or(Symbol::new("k"));
                            let state = cont.get(1).cloned();
                            let op_body = self.desugar_expr(&inner[3])?;
                            clauses.push(HandlerClause {
                                operation: first.clone(),
                                params: op_params,
                                continuation: cont_name,
                                state,
                                body: Box::new(op_body),
                            });
                            i += 1;
                            continue;
                        }
                    }
                    // Effect type specification: (EffectName type_args...)
                    if effect_name.as_str() == "Unknown" {
                        effect_name = first.clone();
                        if inner.len() > 1 {
                            for a in &inner[1..] {
                                type_args.push(self.desugar_type_with_params(a, &[])?);
                            }
                        }
                        i += 1;
                        continue;
                    }
                }
            }
            // Return clause
            if effect_name.as_str() != "Unknown" && clauses.is_empty() {
                let ret_clause = Some(Box::new(self.desugar_expr(&items[i])?));
                let handler = Handler { effect_name, type_args, clauses, return_clause: ret_clause };
                return Ok(CoreExpr::new(CoreExprNode::Handle(Box::new(body), handler), span));
            }
            i += 1;
        }

        let handler = Handler { effect_name, type_args, clauses, return_clause: None };
        Ok(CoreExpr::new(CoreExprNode::Handle(Box::new(body), handler), span))
    }

    fn desugar_perform(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        if items.len() < 2 {
            return Err(DesugarError {
                message: "perform requires operation name".into(),
                span,
            });
        }

        let op_name = match &items[1].node {
            Expr::Sym(s) => s.clone(),
            _ => return Err(DesugarError {
                message: "perform operation must be a symbol".into(),
                span: items[1].span,
            }),
        };

        let mut args = Vec::new();
        for item in &items[2..] {
            args.push(self.desugar_expr(item)?);
        }

        Ok(CoreExpr::new(
            CoreExprNode::Perform(op_name, args),
            span,
        ))
    }

    fn desugar_app(&self, items: &[SExpr], span: Span) -> Result<CoreExpr, DesugarError> {
        let func = self.desugar_expr(&items[0])?;

        if items.len() == 1 {
            return Ok(func);
        }

        // Build left-associative application: (f x y z) = ((f x) y) z
        let mut result = func;
        for item in &items[1..] {
            let arg = self.desugar_expr(item)?;
            result = CoreExpr::new(
                CoreExprNode::App(Box::new(result), Box::new(arg)),
                span,
            );
        }

        Ok(result)
    }
}
