#![feature(decl_macro)]

pub mod compileroptions;
pub mod diagnostics;
pub mod verifier;

/// Unified compiler module.
pub mod ns {
    pub use whackengine_mxmlsemantics::ns::*;
    pub use super::compileroptions::*;
    pub use super::diagnostics::*;
    pub use super::verifier::*;
}