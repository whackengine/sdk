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
    #[serde(rename = "build-dependencies")]
    pub build_dependencies: Option<HashMap<String, ManifestDependency>>,
    #[serde(rename = "build-source")]
    pub build_source: Option<Vec<ManifestSource>>,
    pub javascript: Option<Vec<ManifestJscript>>,
    /// Configuration constants.
    pub define: Option<HashMap<String, toml::Value>>,
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
    #[serde(rename = "license-file")]
    pub license_file: Option<String>,
    pub description: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub metadata: Option<toml::Value>,
}

#[derive(Serialize, Deserialize)]
pub struct ManifestSource {
    pub path: String,
    pub include: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct ManifestClientSide {
    #[serde(rename = "main-class")]
    pub main_class: String,
}

#[derive(Serialize, Deserialize)]
pub struct ManifestServerSide {
    #[serde(rename = "executable-name")]
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
pub struct ManifestJscript {
    path: String,
    #[serde(rename = "import-declaration")]
    import_declaration: String,
    #[serde(rename = "client-side")]
    client_side: Option<bool>,
    #[serde(rename = "server-side")]
    server_side: Option<bool>,
}