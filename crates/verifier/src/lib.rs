#![feature(decl_macro)]

pub mod codegen;
pub mod compileroptions;
pub mod diagnostics;
pub mod verifier;

/// Unified compiler module.
pub mod ns {
    pub use mxmlextrema_mxmlcaot::ns::*;
    pub use super::codegen::*;
    pub use super::compileroptions::*;
    pub use super::diagnostics::*;
    pub use super::verifier::*;
}