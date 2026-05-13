use anyhow::{Context, Result, bail};
use fs_err as fs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct Config {
    pub project: Project,
    #[serde(default)]
    pub artifacts: HashMap<String, Artifact>,
    #[serde(default)]
    pub build_group: HashMap<String, BuildCommand>,
    pub development: Option<Development>,
    #[serde(default)]
    pub preview: Preview,
    #[serde(default)]
    pub release: Release,
    pub catalog: Option<Catalog>,
}

#[derive(Deserialize)]
pub struct Project {
    pub id: String,
    pub name: Option<String>,
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

impl std::default::Default for BuildCommand {
    fn default() -> Self {
        BuildCommand::Multiple(vec![])
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct BuildGroupRef {
    pub group: String,
}

#[derive(Deserialize)]
pub struct Development {
    pub aviutl2_version: String,
    #[serde(default = "Development::default_install_dir")]
    pub install_dir: String,
    #[serde(default = "Development::default_profile")]
    pub profile: String,
    #[serde(default)]
    pub prebuild: BuildCommand,
    #[serde(default)]
    pub postbuild: BuildCommand,
    #[serde(default)]
    pub catalog_dependencies: Vec<CatalogDependency>,
}

impl Development {
    pub fn default_install_dir() -> String {
        ".aviutl2-cli/development".to_string()
    }

    pub fn default_profile() -> String {
        "debug".to_string()
    }
}

#[derive(Deserialize, Clone, PartialEq)]
pub struct Preview {
    pub aviutl2_version: Option<String>,
    #[serde(default = "Preview::default_install_dir")]
    pub install_dir: String,
    #[serde(default = "Preview::default_profile")]
    pub profile: String,
    pub include: Option<Vec<String>>,
    #[serde(default)]
    pub prebuild: BuildCommand,
    #[serde(default)]
    pub postbuild: BuildCommand,
    #[serde(default)]
    pub catalog_dependencies: Vec<CatalogDependency>,
}

impl Default for Preview {
    fn default() -> Self {
        Self {
            aviutl2_version: None,
            install_dir: Self::default_install_dir(),
            profile: Self::default_profile(),
            include: None,
            prebuild: BuildCommand::default(),
            postbuild: BuildCommand::default(),
            catalog_dependencies: Vec::new(),
        }
    }
}

impl Preview {
    pub fn default_install_dir() -> String {
        ".aviutl2-cli/preview".to_string()
    }

    pub fn default_profile() -> String {
        "release".to_string()
    }
}

#[derive(Deserialize, Clone, PartialEq)]
pub struct Release {
    #[serde(default = "Release::default_output_dir")]
    pub output_dir: String,
    pub package_template: Option<String>,
    #[serde(default = "Release::default_package_id")]
    pub package_id: String,
    #[serde(default = "Release::default_package_name")]
    pub package_name: String,
    #[serde(default = "Release::default_package_information")]
    pub package_information: String,
    #[serde(default = "Release::uninstall_subfolder_file")]
    pub uninstall_subfolder_file: bool,
    #[serde(default = "Release::default_zip_name")]
    pub zip_name: String,
    #[serde(default = "Release::default_profile")]
    pub profile: String,
    pub include: Option<Vec<String>>,
    #[serde(default)]
    pub prebuild: BuildCommand,
    #[serde(default)]
    pub postbuild: BuildCommand,
}

impl Default for Release {
    fn default() -> Self {
        Self {
            output_dir: Self::default_output_dir(),
            package_template: None,
            package_id: Self::default_package_id(),
            package_name: Self::default_package_name(),
            package_information: Self::default_package_information(),
            uninstall_subfolder_file: Self::uninstall_subfolder_file(),
            zip_name: Self::default_zip_name(),
            profile: Self::default_profile(),
            include: None,
            prebuild: BuildCommand::default(),
            postbuild: BuildCommand::default(),
        }
    }
}

impl Release {
    pub fn default_output_dir() -> String {
        "release".to_string()
    }

    pub fn default_package_id() -> String {
        "{id}".to_string()
    }

    pub fn default_package_name() -> String {
        "{name}".to_string()
    }

    pub fn default_package_information() -> String {
        "{name} v{version}".to_string()
    }

    pub fn default_zip_name() -> String {
        "{id}-v{version}".to_string()
    }

    pub fn default_profile() -> String {
        "release".to_string()
    }

    pub fn uninstall_subfolder_file() -> bool {
        false
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct Catalog {
    pub id: String,
    pub description_path: Option<String>,
    pub license_path: Option<CatalogLicensePath>,
    pub download_repo: Option<CatalogDownloadRepo>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
#[serde(untagged)]
pub enum CatalogLicensePath {
    Simple(String),
    Detailed(CatalogLicenseDef),
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct CatalogLicenseDef {
    #[serde(rename = "type")]
    pub license_type: crate::catalog_schema::LicenseType,
    pub path: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct CatalogDownloadRepo {
    pub owner: String,
    pub repo: String,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct CatalogDependency {
    pub id: String,
}

impl<'de> Deserialize<'de> for CatalogDependency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum CatalogDependencyDef {
            Simple(String),
            Detailed { id: String },
        }

        let def = CatalogDependencyDef::deserialize(deserializer)?;
        let dependency = match def {
            CatalogDependencyDef::Simple(id) => CatalogDependency { id },
            CatalogDependencyDef::Detailed { id } => CatalogDependency { id },
        };
        Ok(dependency)
    }
}

pub struct ConfigLoadOpts {
    pub patch: Option<String>,
    pub override_path: Option<String>,
}

pub fn load_config(opts: &ConfigLoadOpts) -> Result<Config> {
    if opts.patch.is_some() && opts.override_path.is_some() {
        bail!("-c と -C は同時に指定できません");
    }

    let config_path = if let Some(override_path) = &opts.override_path {
        let path = PathBuf::from(override_path);
        let abs_path = path
            .canonicalize()
            .with_context(|| format!("ファイルが見つかりません: {}", path.display()))?;
        let parent = abs_path.parent().with_context(|| {
            format!("親ディレクトリの取得に失敗しました: {}", abs_path.display())
        })?;
        std::env::set_current_dir(parent).with_context(|| {
            format!(
                "カレントディレクトリの変更に失敗しました: {}",
                parent.display()
            )
        })?;
        abs_path
    } else {
        find_and_cd_to_project()?
    };

    let config = if let Some(patch_path) = &opts.patch {
        let mut base = load_config_from_path(&config_path)
            .context("元の設定ファイルの読み込みに失敗しました")?;
        let patch = load_config_from_path(&PathBuf::from(patch_path))
            .context("パッチファイルの読み込みに失敗しました")?;
        merge_toml(&mut base, patch);
        base
    } else {
        load_config_from_path(&config_path).context("設定ファイルの読み込みに失敗しました")?
    };
    serde_json::from_value(config).context("設定の解析に失敗しました")
}

fn load_config_from_path(path: &std::path::Path) -> Result<serde_json::Value> {
    if path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"))
    {
        let content = fs::read_to_string(path)
            .with_context(|| format!("設定ファイルの読み込みに失敗しました: {}", path.display()))?;
        toml::from_str(&content).with_context(|| "設定ファイルの解析に失敗しました")
    } else if path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pkl"))
    {
        if std::env::var("AU2_USE_PKL_CLI").is_ok() {
            std::process::Command::new("pkl")
                .arg("eval")
                .arg("--format")
                .arg("json")
                .arg(path)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::inherit())
                .output()
                .with_context(|| "pkl コマンドの実行に失敗しました")
                .and_then(|output| {
                    if !output.status.success() {
                        bail!("pkl コマンドがエラーを返しました");
                    }
                    serde_json::from_slice(&output.stdout)
                        .with_context(|| "pkl コマンドの出力の解析に失敗しました")
                })
        } else {
            pollster::block_on(pklr::eval_to_json(path))
                .with_context(|| "設定ファイルの解析に失敗しました")
        }
    } else {
        let content = fs::read_to_string(path)
            .with_context(|| format!("設定ファイルの読み込みに失敗しました: {}", path.display()))?;
        serde_json::from_str(&content).with_context(|| "設定ファイルの解析に失敗しました")
    }
}

fn merge_toml(base: &mut serde_json::Value, patch: serde_json::Value) {
    match (&mut *base, patch) {
        (serde_json::Value::Object(base_table), serde_json::Value::Object(patch_table)) => {
            for (key, patch_val) in patch_table {
                match base_table.get_mut(&key) {
                    Some(base_val) => merge_toml(base_val, patch_val),
                    None => {
                        base_table.insert(key, patch_val);
                    }
                }
            }
        }
        (base, patch) => *base = patch,
    }
}

pub fn find_and_cd_to_project() -> Result<PathBuf> {
    static CANDIDATE_FILES: &[&str] = &[
        "aviutl2.pkl",
        ".aviutl2.pkl",
        "aviutl2.toml",
        ".aviutl2.toml",
        "aviutl2.json",
        ".aviutl2.json",
        ".config/aviutl2.toml",
        ".config/aviutl2.pkl",
        ".config/aviutl2.json",
    ];
    let mut current =
        std::env::current_dir().context("カレントディレクトリの取得に失敗しました")?;
    loop {
        for candidate in CANDIDATE_FILES {
            let candidate_path = current.join(candidate);
            if candidate_path.is_file() {
                std::env::set_current_dir(&current).with_context(|| {
                    format!(
                        "カレントディレクトリの変更に失敗しました: {}",
                        current.display()
                    )
                })?;
                return Ok(candidate_path);
            }
        }
        if !current.pop() {
            bail!("設定ファイルが見つかりませんでした");
        }
    }
}
