use crate::symbol::Symbol;
use crate::types::Type;
use crate::span::Span;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DataDecl {
    pub name: Symbol,
    pub type_params: Vec<Symbol>,
    pub constructors: Vec<Constructor>,
    pub deriving: Vec<Symbol>,
    pub is_hit: bool,
    /// HIT 路径构造器边界声明(§7.4/16.3):显示文本,如 "= loop base"
    pub boundary: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Constructor {
    pub name: Symbol,
    pub fields: Vec<Field>,
    pub gadt_return_type: Option<Type>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: Option<Symbol>,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct DataEnv {
    declarations: HashMap<Symbol, DataDecl>,
}

impl DataEnv {
    pub fn new() -> Self {
        Self {
            declarations: HashMap::new(),
        }
    }

    pub fn register(&mut self, decl: DataDecl) {
        self.declarations.insert(decl.name.clone(), decl);
    }

    pub fn lookup(&self, name: &Symbol) -> Option<&DataDecl> {
        self.declarations.get(name)
    }

    pub fn lookup_constructor(&self, name: &Symbol) -> Option<(&DataDecl, &Constructor)> {
        for decl in self.declarations.values() {
            for ctor in &decl.constructors {
                if &ctor.name == name {
                    return Some((decl, ctor));
                }
            }
        }
        None
    }

    pub fn constructor_type(&self, ctor_name: &Symbol) -> Option<Type> {
        if let Some((decl, ctor)) = self.lookup_constructor(ctor_name) {
            let mut result_type = Type::Con(crate::types::TypeCon {
                name: decl.name.clone(),
                kind: crate::types::Kind::Star,
            });

            for param in &decl.type_params {
                result_type = Type::App(
                    Box::new(result_type),
                    Box::new(Type::Var(crate::types::TypeVar {
                        name: param.clone(),
                        kind: crate::types::Kind::Star,
                        id: 0,
                    })),
                );
            }

            let mut func_type = result_type;
            for field in ctor.fields.iter().rev() {
                func_type = Type::Fun(
                    Box::new(field.ty.clone()),
                    crate::types::FunAnnotation::default(),
                    Box::new(func_type),
                );
            }

            Some(func_type)
        } else {
            None
        }
    }
}
