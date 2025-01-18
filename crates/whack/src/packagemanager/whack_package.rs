use std::path::PathBuf;
use std::rc::Rc;
use whackengine_verifier::ns::{CompilationUnit, Mxml, Program, SharedArray};

pub struct WhackPackage {
    /// Physical path relative to the entry path.
    pub path: PathBuf,
    /// List of ActionScript sources.
    pub sources: SharedArray<WhackSource>,
    /// List of ActionScript build script sources.
    pub build_script_sources: SharedArray<WhackSource>,
}

pub enum WhackSource {
    As3(Rc<Program>),
    Mxml(Rc<Mxml>),
}

impl WhackSource {
    pub fn compilation_unit(&self) -> Rc<CompilationUnit> {
        match self {
            WhackSource::As3(program) => program.location.compilation_unit(),
            WhackSource::Mxml(m) => m.location.compilation_unit(),
        }
    }
}