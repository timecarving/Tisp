use tisp_core::ast::Expr;
use tisp_core::span::{Span, Spanned};
use tisp_core::symbol::Symbol;
use crate::lexer::{Token, SpannedToken};

#[derive(Debug)]
pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {}", self.message, self.span)
    }
}

impl std::error::Error for ParseError {}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse_program(&mut self) -> Result<Vec<Spanned<Expr>>, ParseError> {
        let mut forms = Vec::new();
        while !self.is_eof() {
            forms.push(self.parse_expr()?);
        }
        Ok(forms)
    }

    fn parse_expr(&mut self) -> Result<Spanned<Expr>, ParseError> {
        let tok = self.peek().ok_or_else(|| ParseError {
            message: "unexpected end of input".into(),
            span: Span::dummy(),
        })?;

        match &tok.token {
            Token::LParen => self.parse_list(),
            Token::LBracket => self.parse_vec(),
            Token::LBrace => self.parse_map(),
            Token::Quote => self.parse_prefix(Expr::Quote),
            Token::SyntaxQuote => self.parse_prefix(Expr::SyntaxQuote),
            Token::Unquote => self.parse_prefix(Expr::Unquote),
            Token::UnquoteSplice => self.parse_prefix(Expr::UnquoteSplice),
            Token::Hash => self.parse_set(),
            Token::Int(n) => { let n = *n; let span = self.advance().span; Ok(Spanned::new(Expr::Int(n), span)) }
            Token::Float(f) => { let f = *f; let span = self.advance().span; Ok(Spanned::new(Expr::Float(f), span)) }
            Token::Str(s) => { let s = s.clone(); let span = self.advance().span; Ok(Spanned::new(Expr::Str(s), span)) }
            Token::Char(c) => { let c = *c; let span = self.advance().span; Ok(Spanned::new(Expr::Char(c), span)) }
            Token::Bool(b) => { let b = *b; let span = self.advance().span; Ok(Spanned::new(Expr::Bool(b), span)) }
            Token::Nil => { let span = self.advance().span; Ok(Spanned::new(Expr::Nil, span)) }
            Token::Colon => { let span = self.advance().span; Ok(Spanned::new(Expr::Keyword(Symbol::new(":")), span)) }
            Token::Arrow => { let span = self.advance().span; Ok(Spanned::new(Expr::Keyword(Symbol::new("->")), span)) }
            Token::Pipe => { let span = self.advance().span; Ok(Spanned::new(Expr::Keyword(Symbol::new("|")), span)) }
            Token::Keyword(k) => { let k = Symbol::new(k); let span = self.advance().span; Ok(Spanned::new(Expr::Keyword(k), span)) }
            Token::Ident(name) => { let name = Symbol::new(name); let span = self.advance().span; Ok(Spanned::new(Expr::Sym(name), span)) }
            _ => Err(ParseError {
                message: format!("unexpected token {:?}", tok.token),
                span: tok.span,
            }),
        }
    }

    fn parse_list(&mut self) -> Result<Spanned<Expr>, ParseError> {
        let start = self.advance().span;
        let mut items = Vec::new();
        let mut tail = None;
        while !self.check(&Token::RParen) && !self.is_eof() {
            if self.check(&Token::Dot) {
                self.advance(); // consume dot
                tail = Some(Box::new(self.parse_expr()?));
                break;
            }
            items.push(self.parse_expr()?);
        }
        let end = self.expect(Token::RParen, "expected ')'")?;
        match tail {
            Some(t) => Ok(Spanned::new(Expr::ConsPattern(items, t), start.merge(end))),
            None => Ok(Spanned::new(Expr::List(items), start.merge(end))),
        }
    }

    fn parse_vec(&mut self) -> Result<Spanned<Expr>, ParseError> {
        let start = self.advance().span;
        let mut items = Vec::new();
        let mut tail = None;
        while !self.check(&Token::RBracket) && !self.is_eof() {
            if self.check(&Token::Dot) {
                self.advance(); // consume dot
                tail = Some(Box::new(self.parse_expr()?));
                break;
            }
            items.push(self.parse_expr()?);
        }
        let end = self.expect(Token::RBracket, "expected ']'")?;
        match tail {
            Some(t) => Ok(Spanned::new(Expr::ConsPattern(items, t), start.merge(end))),
            None => Ok(Spanned::new(Expr::Vec(items), start.merge(end))),
        }
    }

    fn parse_map(&mut self) -> Result<Spanned<Expr>, ParseError> {
        let start = self.advance().span;
        let mut pairs = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_eof() {
            let key = self.parse_expr()?;
            let val = self.parse_expr()?;
            pairs.push((key, val));
        }
        let end = self.expect(Token::RBrace, "expected '}'")?;
        Ok(Spanned::new(Expr::Map(pairs), start.merge(end)))
    }

    fn parse_set(&mut self) -> Result<Spanned<Expr>, ParseError> {
        let start = self.advance().span;
        self.expect(Token::LBrace, "expected '{' after '#'")?;
        let mut items = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_eof() {
            items.push(self.parse_expr()?);
        }
        let end = self.expect(Token::RBrace, "expected '}'")?;
        Ok(Spanned::new(Expr::Set(items), start.merge(end)))
    }

    fn parse_prefix(&mut self, wrap: fn(Box<Spanned<Expr>>) -> Expr) -> Result<Spanned<Expr>, ParseError> {
        let start = self.advance().span;
        let inner = self.parse_expr()?;
        let span = start.merge(inner.span);
        Ok(Spanned::new(wrap(Box::new(inner)), span))
    }

    fn peek(&self) -> Option<&SpannedToken> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> SpannedToken {
        let tok = self.tokens[self.pos].clone();
        self.pos += 1;
        tok
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn check(&self, expected: &Token) -> bool {
        self.peek().map_or(false, |t| std::mem::discriminant(&t.token) == std::mem::discriminant(expected))
    }

    fn expect(&mut self, expected: Token, msg: &str) -> Result<Span, ParseError> {
        if self.check(&expected) {
            Ok(self.advance().span)
        } else {
            let span = self.peek().map_or(Span::dummy(), |t| t.span);
            Err(ParseError { message: msg.into(), span })
        }
    }
}

pub fn parse(input: &str) -> Result<Vec<Spanned<Expr>>, ParseError> {
    let tokens = crate::lexer::tokenize(input).map_err(|e| ParseError {
        message: e.to_string(),
        span: e.span,
    })?;
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_int() {
        let forms = parse("42").unwrap();
        assert_eq!(forms.len(), 1);
        assert!(matches!(forms[0].node, Expr::Int(42)));
    }

    #[test] fn test_float() {
        let forms = parse("3.14").unwrap();
        assert_eq!(forms.len(), 1);
        assert!(matches!(forms[0].node, Expr::Float(f) if (f - 3.14).abs() < 0.01));
    }

    #[test] fn test_string() {
        let forms = parse(r#""hello""#).unwrap();
        assert_eq!(forms.len(), 1);
        assert!(matches!(forms[0].node, Expr::Str(ref s) if s == "hello"));
    }

    #[test] fn test_symbol() {
        let forms = parse("foo").unwrap();
        assert_eq!(forms.len(), 1);
        assert!(matches!(forms[0].node, Expr::Sym(ref s) if s.as_str() == "foo"));
    }

    #[test] fn test_list() {
        let forms = parse("(+ 1 2)").unwrap();
        assert_eq!(forms.len(), 1);
        assert!(matches!(forms[0].node, Expr::List(_)));
    }

    #[test] fn test_vector() {
        let forms = parse("[1 2 3]").unwrap();
        assert_eq!(forms.len(), 1);
        assert!(matches!(forms[0].node, Expr::Vec(_)));
    }

    #[test] fn test_map() {
        let forms = parse("{:a 1 :b 2}").unwrap();
        assert_eq!(forms.len(), 1);
        assert!(matches!(forms[0].node, Expr::Map(_)));
    }

    #[test] fn test_keyword() {
        let forms = parse(":name").unwrap();
        assert_eq!(forms.len(), 1);
        assert!(matches!(forms[0].node, Expr::Keyword(ref s) if s.as_str() == "name"));
    }

    #[test] fn test_bool() {
        let forms = parse("true false").unwrap();
        assert_eq!(forms.len(), 2);
        assert!(matches!(forms[0].node, Expr::Bool(true)));
        assert!(matches!(forms[1].node, Expr::Bool(false)));
    }

    #[test] fn test_nil() {
        let forms = parse("nil").unwrap();
        assert_eq!(forms.len(), 1);
        assert!(matches!(forms[0].node, Expr::Nil));
    }

    #[test] fn test_defn() {
        let forms = parse("(defn add [x y] (+ x y))").unwrap();
        assert_eq!(forms.len(), 1);
    }

    #[test] fn test_defn_with_types() {
        let forms = parse("(defn add [x : i64 y : i64] -> i64 (+ x y))").unwrap();
        assert_eq!(forms.len(), 1);
    }
}

