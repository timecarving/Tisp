pub mod region;
pub mod persistent;
pub mod effect;
pub mod logic;
pub mod constraint;
pub mod abduction;
pub mod concurrent;
pub mod process;
pub mod hott;
pub mod depgraded;
pub mod frp;
pub mod metaprogram;
pub mod theorem;
pub mod stdlib;

/// Re-export key types
pub use region::RegionStack;
pub use persistent::PersistentValue;
pub use effect::EffectRuntime;
