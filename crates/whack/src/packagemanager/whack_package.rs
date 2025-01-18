use std::{collections::HashMap, path::PathBuf};
use std::rc::Rc;
use whackengine_verifier::ns::{shared_array, CompilationUnit, Mxml, Program, SharedArray};
use crate::packagemanager::*;

pub struct WhackPackage {
    /// Absolute physical path.
    pub absolute_path: PathBuf,
    /// Physical path relative to the entry point path.
    pub relative_path: String,
    /// The manifest file representing the package.
    pub manifest: WhackManifest,
    /// List of ActionScript sources.
    pub sources: SharedArray<WhackSource>,
    /// List of ActionScript build script sources.
    pub build_script_sources: SharedArray<WhackSource>,
}

#[derive(Clone)]
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

pub struct WhackPackageInternator {
    m_by_relative_path: HashMap<String, Rc<WhackPackage>>,
}

impl WhackPackageInternator {
    pub fn new() -> Self {
        WhackPackageInternator {
            m_by_relative_path: HashMap::new(),
        }
    }

    pub fn intern(&mut self, absolute_path: &PathBuf, relative_path: &str, manifest: &WhackManifest) -> Rc<WhackPackage> {
        let r = self.m_by_relative_path.get(relative_path);
        if let Some(r) = r {
            return r.clone();
        }

        let r = Rc::new(WhackPackage {
            absolute_path: absolute_path.clone(),
            relative_path: relative_path.to_owned(),
            manifest: manifest.clone(),
            sources: shared_array![],
            build_script_sources: shared_array![],
        });
        self.m_by_relative_path.insert(relative_path.to_owned(), r.clone());
        r
    }
}