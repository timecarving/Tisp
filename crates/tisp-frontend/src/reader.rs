pub fn read(input: &str) -> Result<Vec<tisp_core::ast::SExpr>, crate::parser::ParseError> {
    crate::parser::parse(input)
}
