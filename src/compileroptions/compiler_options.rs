use crate::ns::*;

#[derive(Clone)]
pub struct CompilerOptions {
    pub warnings: CompilerWarningOptions,
    /// Used for identifying the AS3 package in a MXML source tree.
    pub source_path: Vec<String>,
}

impl CompilerOptions {
    pub fn of(cu: &Rc<CompilationUnit>) -> Rc<CompilerOptions> {
        Rc::downcast(cu.compiler_options().expect("Compiler options missing for a CompilationUnit."))
            .expect("Wrong assigned compiler options.")
    }
}

#[derive(Clone)]
#[non_exhaustive]
pub struct CompilerWarningOptions {
    pub unused: bool,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            warnings: Default::default(),
            source_path: vec![],
        }
    }
}

impl Default for CompilerWarningOptions {
    fn default() -> Self {
        Self {
            unused: true,
        }
    }
}