use crate::ns::*;
use std::cell::Cell;

pub struct CodegenClassInfo {
    pub slots: Cell<usize>,
    /// Variable slot order for that specific class
    /// (excludes variable slots from base classes).
    pub slot_vars: SharedArray<Entity>,
}