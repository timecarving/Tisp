use crate::symbol::Symbol;
use crate::types::Mode;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InstTree {
    pub kind: InstKind,
    pub children: Vec<(Symbol, InstTree)>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum InstKind {
    Free,
    Bound,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ModeMapping {
    pub from: Mode,
    pub to: Mode,
}

pub fn mode_is_input(m: &Mode) -> bool {
    matches!(m, Mode::In | Mode::Ground)
}

pub fn mode_is_output(m: &Mode) -> bool {
    matches!(m, Mode::Out | Mode::Free)
}

pub fn mode_compose(a: &Mode, b: &Mode) -> Mode {
    Mode::Product(Box::new(a.clone()), Box::new(b.clone()))
}
