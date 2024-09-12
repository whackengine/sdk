use semver::Version;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct WhackManifest {
    pub workspace: Option<WorkspaceManifest>,
    pub package: Option<PackageManifest>,
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
    pub source: Option<Vec<PackageManifestSource>>,
    #[serde(rename = "client-side")]
    pub client_side: Option<PackageClientSide>,
    #[serde(rename = "server-side")]
    pub server_side: Option<PackageServerSide>,
}

#[derive(Serialize, Deserialize)]
pub struct PackageManifestSource {
    pub path: String,
    pub include: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct PackageClientSide {
    pub enable: bool,
    #[serde(rename = "main-class")]
    pub main_class: String,
}

#[derive(Serialize, Deserialize)]
pub struct PackageServerSide {
    pub enable: bool,
    #[serde(rename = "main-class")]
    pub main_class: String,
    pub executable_name: Option<String>,
}