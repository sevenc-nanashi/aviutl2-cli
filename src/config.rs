use anyhow::{Context, Result, bail};
use fs_err as fs;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct Config {
    pub project: Project,
    pub artifacts: HashMap<String, Artifact>,
    pub development: Option<Development>,
    pub preview: Option<Preview>,
    pub release: Option<Release>,
}

#[derive(Deserialize)]
pub struct Project {
    pub name: String,
    pub version: String,
}

#[derive(Deserialize)]
pub struct Artifact {
    pub enabled: Option<bool>,
    pub source: Option<String>,
    pub destination: String,
    pub build: Option<BuildCommand>,
    pub placement_method: Option<PlacementMethod>,
    pub profiles: Option<HashMap<String, ArtifactProfile>>,
}

#[derive(Deserialize)]
pub struct ArtifactProfile {
    pub enabled: Option<bool>,
    pub source: Option<String>,
    pub build: Option<BuildCommand>,
}

#[derive(Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PlacementMethod {
    Symlink,
    Copy,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
pub enum BuildCommand {
    Single(String),
    Multiple(Vec<String>),
}

impl BuildCommand {
    pub fn as_vec(&self) -> Vec<String> {
        match self {
            BuildCommand::Single(cmd) => vec![cmd.clone()],
            BuildCommand::Multiple(cmds) => cmds.clone(),
        }
    }
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
