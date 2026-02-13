use anyhow::{Context, Result, bail};
use fs_err as fs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct Config {
    pub project: Project,
    pub artifacts: HashMap<String, Artifact>,
    pub build_group: Option<HashMap<String, BuildCommand>>,
    pub development: Option<Development>,
    pub preview: Option<Preview>,
    pub release: Option<Release>,
    pub catalog: Option<Catalog>,
}

#[derive(Deserialize)]
pub struct Project {
    pub name: String,
    pub version: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct Artifact {
    pub enabled: Option<bool>,
    pub source: Option<String>,
    pub destination: String,
    pub build: Option<BuildCommand>,
    pub placement_method: Option<PlacementMethod>,
    pub profiles: Option<HashMap<String, ArtifactProfile>>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct ArtifactProfile {
    pub enabled: Option<bool>,
    pub source: Option<String>,
    pub build: Option<BuildCommand>,
}

#[derive(Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PlacementMethod {
    Symlink,
    Copy,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum BuildCommand {
    Single(String),
    Multiple(Vec<String>),
    Group(BuildGroupRef),
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct BuildGroupRef {
    pub group: String,
}

#[derive(Deserialize)]
pub struct Development {
    pub aviutl2_version: String,
    pub install_dir: Option<String>,
    pub profile: Option<String>,
    pub prebuild: Option<BuildCommand>,
    pub postbuild: Option<BuildCommand>,
}

#[derive(Deserialize)]
pub struct Preview {
    pub aviutl2_version: Option<String>,
    pub install_dir: Option<String>,
    pub profile: Option<String>,
    pub include: Option<Vec<String>>,
    pub prebuild: Option<BuildCommand>,
    pub postbuild: Option<BuildCommand>,
}

#[derive(Deserialize)]
pub struct Release {
    pub output_dir: Option<String>,
    pub package_template: Option<String>,
    pub zip_name: Option<String>,
    pub profile: Option<String>,
    pub include: Option<Vec<String>>,
    pub prebuild: Option<BuildCommand>,
    pub postbuild: Option<BuildCommand>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct Catalog {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub catalog_type: CatalogType,
    pub author: String,
    pub original_author: Option<String>,
    pub niconi_commons_id: Option<String>,
    pub summary: String,
    pub homepage: String,
    pub dependencies: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub description: CatalogDescription,
    pub license: CatalogLicense,
    pub download_source: CatalogDownloadSource,
    pub install_steps: Option<Vec<CatalogAction>>,
    pub uninstall_steps: Option<Vec<CatalogAction>>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CatalogType {
    Output,
    Input,
    Filter,
    Common,
    Modification,
    Script,
    Language,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CatalogLicenseText {
    Inline { content: String },
    File { path: String },
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum CatalogDescription {
    Plain(String),
    Url(CatalogDescriptionUrl),
    Inline(CatalogDescriptionInline),
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct CatalogDescriptionUrl {
    #[serde(rename = "type")]
    pub description_type: CatalogDescriptionType,
    pub url: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct CatalogDescriptionInline {
    #[serde(rename = "type")]
    pub description_type: CatalogDescriptionType,
    pub content: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CatalogDescriptionType {
    Url,
    Inline,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum CatalogLicense {
    Template(TemplateCatalogLicense),
    Custom(CustomCatalogLicense),
    Cc0(CC0License),
    Other(OtherCatalogLicense),
    Unknown(UnknownCatalogLicense),
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct TemplateCatalogLicense {
    #[serde(rename = "type")]
    pub license_type: TemplateCatalogLicenseType,
    pub template: serde_constant::ConstBool<true>,
    pub author: String,
    pub year: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct CustomCatalogLicense {
    #[serde(rename = "type")]
    pub license_type: TemplateCatalogLicenseType,
    pub template: serde_constant::ConstBool<false>,
    pub text: CatalogLicenseText,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct CC0License {
    #[serde(rename = "type")]
    pub license_type: CC0LicenseType,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct OtherCatalogLicense {
    #[serde(rename = "type")]
    pub license_type: OtherCatalogLicenseType,
    pub name: Option<String>,
    pub text: CatalogLicenseText,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct UnknownCatalogLicense {
    #[serde(rename = "type")]
    pub license_type: UnknownCatalogLicenseType,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub enum TemplateCatalogLicenseType {
    #[serde(rename = "MIT")]
    Mit,
    #[serde(rename = "Apache-2.0")]
    Apache20,
    #[serde(rename = "BSD-2-Clause")]
    Bsd2Clause,
    #[serde(rename = "BSD-3-Clause")]
    Bsd3Clause,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub enum CC0LicenseType {
    #[serde(rename = "CC0-1.0")]
    Cc0,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub enum OtherCatalogLicenseType {
    #[serde(rename = "other")]
    Other,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub enum UnknownCatalogLicenseType {
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum CatalogDownloadSource {
    #[serde(rename = "direct")]
    Direct { url: String },
    #[serde(rename = "booth")]
    Booth { url: String },
    #[serde(rename = "github")]
    Github {
        owner: String,
        repo: String,
        pattern: Option<String>,
    },
    #[serde(rename = "google_drive")]
    GoogleDrive { id: String },
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
#[serde(tag = "action")]
pub enum CatalogAction {
    #[serde(rename = "download")]
    Download,
    #[serde(rename = "extract")]
    Extract,
    #[serde(rename = "copy")]
    Copy { from: String, to: String },
    #[serde(rename = "delete")]
    Delete { path: String },
    #[serde(rename = "run")]
    Run {
        path: String,
        args: Vec<String>,
        elevate: Option<bool>,
    },
}

pub fn load_config() -> Result<Config> {
    let path = find_config_path()?;
    let content = fs::read_to_string(&path)
        .with_context(|| format!("設定ファイルの読み込みに失敗しました: {}", path.display()))?;
    toml::from_str(&content).with_context(|| "設定ファイルの解析に失敗しました")
}

fn find_config_path() -> Result<PathBuf> {
    let local = PathBuf::from("aviutl2.toml");
    if local.exists() {
        return Ok(local);
    }
    let config_dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .context("設定ファイルが見つかりません")?;
    let path = config_dir.join("aviutl2.toml");
    if path.exists() {
        return Ok(path);
    }
    bail!("aviutl2.toml が見つかりません");
}
