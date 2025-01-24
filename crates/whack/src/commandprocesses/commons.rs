use std::{path::PathBuf, str::FromStr};
use crate::packagemanager::*;
use colored::Colorize;
use hydroperfox_filepaths::FlexPath;
use whackengine_verifier::ns::*;

pub struct CommandProcessCommons;

impl CommandProcessCommons {
    /// Returns (dir, lockfile, lockfile_path, found_base_manifest).
    pub fn entry_point_lookup(dir: &PathBuf) -> (FlexPath, Option<WhackLockfile>, PathBuf, bool) {
        let mut dir = FlexPath::new_native(dir.to_str().unwrap());
        let mut lockfile: Option<WhackLockfile> = None;
        let lockfile_path = PathBuf::from_str(&dir.resolve("whack.lock").to_string_with_flex_separator()).unwrap();
        let mut found_base_manifest = false;
        loop {
            let manifest_path = PathBuf::from_str(&dir.resolve("whack.toml").to_string_with_flex_separator()).unwrap();
    
            if std::fs::exists(&manifest_path).unwrap() && std::fs::metadata(&manifest_path).unwrap().is_file() {
                found_base_manifest = true;
    
                if std::fs::exists(&lockfile_path).unwrap() && std::fs::metadata(&lockfile_path).unwrap().is_file() {
                    lockfile = toml::from_str::<WhackLockfile>(&std::fs::read_to_string(&lockfile_path).unwrap()).ok();
                }
    
                break;
            }
    
            // Look up
            let next_dir = dir.resolve("..");
            if dir == next_dir || next_dir.to_string().is_empty() {
                break;
            }
            dir = next_dir;
        }

        (dir, lockfile, lockfile_path, found_base_manifest)
    }

    pub fn print_package_processing_error(error: WhackPackageProcessingError) {
        match error {
            WhackPackageProcessingError::ManifestNotFound => {
                println!("{} {}", "Error:".red(), "Whack manifest not found.");
            },
            WhackPackageProcessingError::PackageMustBeSpecified => {
                println!("{} {}", "Error:".red(), "Package must be specified.");
            },
            WhackPackageProcessingError::CircularDependency { directory } => {
                println!("{} Circular dependency is not allowed: {}", "Error:".red(), directory);
            },
            WhackPackageProcessingError::InvalidManifest { manifest_path, message } => {
                println!("{} Whack manifest at {} contains invalid TOML: {}", "Error:".red(), manifest_path, message);
            },
            WhackPackageProcessingError::UnspecifiedWorkspaceMember => {
                println!("{} Must specify which package to be processed in Whack workspace.", "Error:".red());
            },
            WhackPackageProcessingError::ManifestIsNotAPackage { manifest_path } => {
                println!("{} Whack manifest at {} does not describe a package.", "Error:".red(), manifest_path);
            },
            WhackPackageProcessingError::IllegalPackageName { name } => {
                println!("{} Found illegal package name: {}", "Error:".red(), name);
            },
            WhackPackageProcessingError::FileNotFound { path } => {
                println!("{} File not found: {}", "Error:".red(), path);
            },
            WhackPackageProcessingError::UnrecognizedSourceFileExtension { path } => {
                println!("{} Unrecognized source file extension at: {}", "Error:".red(), path);
            },
        }
    }

    pub fn recurse_source_files(path: &PathBuf) -> Result<Vec<Rc<CompilationUnit>>, WhackPackageProcessingError> {
        if !std::fs::exists(path).unwrap() {
            return Err(WhackPackageProcessingError::FileNotFound {
                path: path.to_str().unwrap().to_owned(),
            });
        }
        let m = std::fs::metadata(path).unwrap();
        if m.is_file() {
            let flexpath = FlexPath::new_native(path.to_str().unwrap());
            if !flexpath.has_extensions([".as", ".mxml"]) {
                return Err(WhackPackageProcessingError::UnrecognizedSourceFileExtension {
                    path: path.to_str().unwrap().to_owned(),
                });
            }
            let text = std::fs::read_to_string(path).unwrap();
            return Ok(vec![CompilationUnit::new(Some(path.canonicalize().unwrap().to_str().unwrap().to_owned()), text)]);
        }
        if m.is_dir() {
            let mut r: Vec<Rc<CompilationUnit>> = vec![];
            for filename in std::fs::read_dir(path).unwrap() {
                let subpath = filename.unwrap().path();
                let m = std::fs::metadata(&subpath).unwrap();
                if m.is_dir() {
                    r.extend(CommandProcessCommons::recurse_source_files(&subpath)?);
                    continue;
                }
                let subpath_str = subpath.to_str().unwrap();
                if subpath_str.ends_with(".include.as") {
                    continue;
                }
                if subpath_str.ends_with(".as") || subpath_str.ends_with(".mxml") {
                    let text = std::fs::read_to_string(&subpath).unwrap();
                    r.push(CompilationUnit::new(Some(subpath.canonicalize().unwrap().to_str().unwrap().to_owned()), text));
                }
            }
            return Ok(r);
        }
        Ok(vec![])
    }

    pub fn verify_sources_from_dag(dag: &Dag, defined_constants: &Vec<(String, String)>) -> (Rc<Database>, Verifier) {
        let as3host = Rc::new(Database::new(DatabaseOptions {
            project_path: Some(dag.last.absolute_path.canonicalize().unwrap().to_str().unwrap().to_owned()),
            ..default()
        }));

        let mut verifier = Verifier::new(&as3host);

        // Define RT::client and RT::server
        let mut rt_client = true;
        let mut rt_server: bool = false;
        let entry_pckg = dag.last.clone();
        if entry_pckg.manifest.client_side.is_some() {
            if entry_pckg.manifest.server_side.is_some() {
                println!("{} Package cannot be both a client-side and server-side application at the same time.", "Error:".red());
                std::process::exit(1);
            }
            rt_server = false;
        } else if entry_pckg.manifest.server_side.is_some() {
            rt_client = false;
            rt_server = true;
        }

        let mut unused_start = 0usize;

        for pckg in dag.iter() {
            // Setup configuration constants
            as3host.config_constants().set("RT::client".to_owned(), rt_client.to_string());
            as3host.config_constants().set("RT::server".to_owned(), rt_server.to_string());
            for (k, v) in defined_constants.iter() {
                as3host.config_constants().set(k.clone(), v.clone());
            }
            if let Some(define_1) = pckg.manifest.define.as_ref() {
                for (k, v) in define_1.iter() {
                    let val = match v {
                        toml::Value::Boolean(v) => v.to_string(),
                        toml::Value::Float(v) => v.to_string(),
                        toml::Value::Integer(v) => v.to_string(),
                        toml::Value::String(v) => v.clone(),
                        _ => "".to_owned(),
                    };
                    as3host.config_constants().set(k.clone(), val);
                }
            }

            let mut compilation_units: Vec<Rc<CompilationUnit>> = vec![];
            if let Some(source_path) = pckg.manifest.package.as_ref().unwrap().source_path.as_ref() {
                for source_path_1 in source_path.iter() {
                    let source_path_1_str = FlexPath::new_native(pckg.absolute_path.to_str().unwrap()).resolve(source_path_1).to_string_with_flex_separator();
                    match CommandProcessCommons::recurse_source_files(&PathBuf::from_str(&source_path_1_str).unwrap()) {
                        Ok(files) => {
                            compilation_units.extend(files);
                        },
                        Err(error) => {
                            CommandProcessCommons::print_package_processing_error(error);
                            std::process::exit(1);
                        },
                    }
                }
            }

            // Build the default compiler options
            let compiler_options = Rc::new(CompilerOptions::default());

            // Parse and initialize compiler options across compilation units
            let mut programs: Vec<Rc<Program>> = vec![];
            let mut mxml: Vec<Rc<Mxml>> = vec![];
            for cu in compilation_units.iter() {
                // Initialize compiler options
                cu.set_compiler_options(Some(compiler_options.clone()));

                let file_path = cu.file_path().unwrap();
                if file_path.ends_with(".mxml") {
                    // Parse MXML
                    mxml.push(ParserFacade(cu, default()).parse_mxml());

                    // @todo Remove this when MXML is implemented in verification, codegen,
                    // IDE integration and ASDoc.
                    println!("{} MXML is not implemented yet: {}", "Error:".red(), file_path);
                    std::process::exit(1);
                } else {
                    // Parse AS3
                    programs.push(ParserFacade(cu, default()).parse_program());
                }
            }

            // Contribute WhackSources to the WhackPackage.
            for program in programs.iter() {
                pckg.sources.clone().push(WhackSource::As3(program.clone()));
            }
            for mxml1 in mxml.iter() {
                pckg.sources.clone().push(WhackSource::Mxml(mxml1.clone()));
            }

            // Verify
            verifier.verify_programs(&compiler_options, programs, mxml);

            // Clean arena
            as3host.clean_arena();

            // Report unused
            for entity in Unused(&as3host).all()[unused_start..].iter() {
                let loc = entity.location().unwrap();
                let cu = loc.compilation_unit();
                if CompilerOptions::of(&cu).warnings.unused {
                    let diag: Diagnostic;
                    if entity.is::<PackagePropertyImport>() || entity.is::<PackageWildcardImport>()
                    || entity.is::<PackageRecursiveImport>() {
                        diag = WhackDiagnostic::new_warning(&loc, WhackDiagnosticKind::UnusedImport, diagarg![]);
                    // Nominal entity
                    } else {
                        let name = entity.name().to_string();
                        diag = WhackDiagnostic::new_warning(&loc, WhackDiagnosticKind::Unused, diagarg![name.clone()]);
                    }
                    cu.add_diagnostic(diag.clone());
                    // println!("{} {}", "Warning:".yellow(), WhackDiagnostic(&diag).format_english());
                    // cu.sort_diagnostics();
                }
            }

            unused_start = Unused(&as3host).all().len();

            // Sort and log diagnostics
            for cu in compilation_units.iter() {
                cu.sort_diagnostics();
                for diagnostic in cu.nested_diagnostics() {
                    let diagnostic = WhackDiagnostic(&diagnostic);
                    if diagnostic.is_error() {
                        println!("{} {}", "Error:".red(), diagnostic.format_english());
                    } else {
                        println!("{} {}", "Warning:".yellow(), diagnostic.format_english());
                    }

                    println!("\n{}\n", &diagnostic.location().show_code());
                }
            }

            // Clear configuration constants
            as3host.clear_config_constants();

            // If there are any errors, stop verification from here.
            if verifier.invalidated() {
                break;
            }
        }

        (as3host, verifier)
    }
}

pub enum WhackPackageProcessingError {
    ManifestNotFound,
    PackageMustBeSpecified,
    CircularDependency {
        directory: String,
    },
    InvalidManifest {
        manifest_path: String,
        message: String,
    },
    UnspecifiedWorkspaceMember,
    ManifestIsNotAPackage {
        manifest_path: String,
    },
    IllegalPackageName {
        name: String,
    },
    FileNotFound {
        path: String,
    },
    UnrecognizedSourceFileExtension {
        path: String
    },
}