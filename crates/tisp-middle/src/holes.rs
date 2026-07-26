
use tisp_core::symbol::Symbol;
use tisp_core::types::Type;
use tisp_core::span::Span;

/// A typed hole — a placeholder that reports the expected type
#[derive(Debug, Clone)]
pub struct Hole {
    pub name: Option<Symbol>,
    pub expected_type: Option<Type>,
    pub span: Span,
}

/// Hole environment — collects all holes found during type checking
#[derive(Debug, Clone)]
pub struct HoleEnv {
    pub holes: Vec<Hole>,
}

impl HoleEnv {
    pub fn new() -> Self {
        Self { holes: Vec::new() }
    }

    pub fn add_hole(&mut self, name: Option<Symbol>, expected_type: Option<Type>, span: Span) {
        self.holes.push(Hole { name, expected_type, span });
    }

    pub fn report(&self) -> String {
        if self.holes.is_empty() {
            return String::new();
        }
        let mut report = String::from("Typed holes found:\n");
        for (i, hole) in self.holes.iter().enumerate() {
            let name = hole.name.as_ref().map_or("_".into(), |s| s.as_str().to_string());
            let ty = hole.expected_type.as_ref()
                .map_or("unknown".to_string(), |t| format!("{:?}", t));
            report.push_str(&format!("  {}: ?{} : {} at {}\n", i + 1, name, ty, hole.span));
        }
        report
    }

    pub fn len(&self) -> usize {
        self.holes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.holes.is_empty()
    }
}
