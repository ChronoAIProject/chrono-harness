//! Structured diagnostics. Callers explicitly supply producer identity and context.
pub use verus_builtin_macros::verus;
pub mod error;
pub mod logging;
pub mod rolling;
pub mod runtime;
pub mod verified_rolling;
pub mod verified_runtime;
