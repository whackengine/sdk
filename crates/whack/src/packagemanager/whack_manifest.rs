use std::collections::HashMap;
use semver::Version;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct WhackManifest {
    pub workspace: Option<WorkspaceManifest>,
    pub package: Option<PackageManifest>,
    pub source: Option<Vec<ManifestSource>>,
    #[serde(rename = "client-side")]
    pub client_side: Option<ManifestClientSide>,
    #[serde(rename = "server-side")]
    pub server_side: Option<ManifestServerSide>,
    pub dependencies: Option<HashMap<String, ManifestDependency>>,
    pub build_script: Option<ManifestBuildScript>,
    pub js: Option<Vec<ManifestJscript>>,
    /// Configuration constants.
    pub define: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize)]
pub struct WorkspaceManifest {
    pub members: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PackageManifest {
    pub name: String,
    pub version: Version,
    pub author: Option<String>,
    pub license: Option<String>,
    pub description: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
pub struct ManifestSource {
    pub path: String,
    pub include: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct ManifestClientSide {
    pub enable: bool,
    #[serde(rename = "main-class")]
    pub main_class: String,
}

#[derive(Serialize, Deserialize)]
pub struct ManifestServerSide {
    pub enable: bool,
    pub executable_name: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum ManifestDependency {
    Version(Version),
    Advanced {
        version: Option<Version>,
        path: Option<String>,
        git: Option<String>,
    },
}

#[derive(Serialize, Deserialize)]
pub struct ManifestBuildScript {
    pub source: Option<Vec<ManifestSource>>,
}

#[derive(Serialize, Deserialize)]
pub struct ManifestJscript {
    path: String,
    #[serde(rename = "import-declaration")]
    import_declaration: String,
}