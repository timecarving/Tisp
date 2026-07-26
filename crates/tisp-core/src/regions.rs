use crate::symbol::Symbol;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Region {
    Var(RegionId),
    Parent(Box<Region>),
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RegionId {
    pub name: Symbol,
    pub id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum RegionKind {
    Finite,
    Infinite,
    Scalar,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RegionInfo {
    pub id: RegionId,
    pub kind: RegionKind,
    pub multiplicity: RegionMultiplicity,
    pub runtime_type: RuntimeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum RegionMultiplicity {
    Zero,
    One,
    Infinite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum RuntimeType {
    Real,
    String,
    Top,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum StorageMode {
    AtTop,
    AtBot,
    Sat,
}
