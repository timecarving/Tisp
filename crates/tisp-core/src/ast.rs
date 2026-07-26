use crate::span::Spanned;
use crate::symbol::Symbol;

pub type SExpr = Spanned<Expr>;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Expr {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Char(char),
    Keyword(Symbol),
    Sym(Symbol),
    List(Vec<SExpr>),
    Vec(Vec<SExpr>),
    Map(Vec<(SExpr, SExpr)>),
    Set(Vec<SExpr>),
    ConsPattern(Vec<SExpr>, Box<SExpr>),
    Quote(Box<SExpr>),
    SyntaxQuote(Box<SExpr>),
    Unquote(Box<SExpr>),
    UnquoteSplice(Box<SExpr>),
}

impl Expr {
    pub fn sym(name: &str) -> Self {
        Expr::Sym(Symbol::new(name))
    }

    pub fn kw(name: &str) -> Self {
        Expr::Keyword(Symbol::new(name))
    }

    pub fn list(items: Vec<SExpr>) -> Self {
        Expr::List(items)
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Expr::Nil => false,
            Expr::Bool(false) => false,
            _ => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Program {
    pub forms: Vec<SExpr>,
}
