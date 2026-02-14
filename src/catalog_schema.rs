use serde::{Deserialize, Serialize};

/* ---------- primitives ---------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Copyright {
    pub years: String,
    pub holder: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    #[serde(rename = "type")]
    pub license_type: String,

    #[serde(rename = "isCustom")]
    pub is_custom: bool,

    pub copyrights: Vec<Copyright>,

    #[serde(rename = "licenseBody")]
    pub license_body: Option<String>,
}

/* ---------- installer source ---------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubSource {
    pub owner: String,
    pub repo: String,
    pub pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleDriveSource {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InstallerSource {
    Direct {
        direct: String,
    },
    Booth {
        booth: String,
    },
    Github {
        github: GithubSource,
    },
    GoogleDrive {
        #[serde(rename = "GoogleDrive")]
        google_drive: GoogleDriveSource,
    },
}

/* ---------- installer actions ---------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum InstallerAction {
    #[serde(rename = "download")]
    Download {},

    #[serde(rename = "extract")]
    Extract {},

    #[serde(rename = "extract_sfx")]
    ExtractSfx {},

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

    #[serde(rename = "run_auo_setup")]
    RunAuoSetup { path: String },
}

/* ---------- installer ---------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Installer {
    pub source: InstallerSource,
    pub install: Vec<InstallerAction>,
    pub uninstall: Vec<InstallerAction>,
}

/* ---------- version ---------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionFile {
    pub path: String,

    #[serde(rename = "XXH3_128")]
    pub xxh3_128: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    pub version: String,
    pub release_date: String,
    pub file: Vec<VersionFile>,
}

/* ---------- image ---------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub thumbnail: Option<String>,

    #[serde(rename = "infoImg")]
    pub info_img: Option<Vec<String>>,
}

/* ---------- catalog entry ---------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,

    #[serde(rename = "type")]
    pub entry_type: CatalogEntryType,

    pub summary: String,
    pub description: String,
    pub author: String,
    #[serde(rename = "originalAuthor")]
    pub original_author: Option<String>,

    #[serde(rename = "repoURL")]
    pub repo_url: String,

    pub licenses: Vec<License>,

    #[serde(rename = "niconiCommonsId")]
    pub niconi_commons_id: Option<String>,

    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
    pub images: Vec<Image>,

    pub installer: Installer,
    pub version: Vec<Version>,
}

#[derive(Debug, Clone)]
pub enum CatalogEntryType {
    AviUtl2,
    Output,
    Input,
    Filter,
    Common,
    Modification,
    Script,
    Other,

    Custom(String),
}
impl<'de> Deserialize<'de> for CatalogEntryType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "本体" => CatalogEntryType::AviUtl2,
            "出力プラグイン" => CatalogEntryType::Output,
            "入力プラグイン" => CatalogEntryType::Input,
            "フィルタプラグイン" => CatalogEntryType::Filter,
            "汎用プラグイン" => CatalogEntryType::Common,
            "MOD" => CatalogEntryType::Modification,
            "スクリプト" => CatalogEntryType::Script,
            "その他" => CatalogEntryType::Other,
            custom => CatalogEntryType::Custom(custom.to_string()),
        })
    }
}
impl Serialize for CatalogEntryType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = match self {
            CatalogEntryType::AviUtl2 => "本体",
            CatalogEntryType::Output => "出力プラグイン",
            CatalogEntryType::Input => "入力プラグイン",
            CatalogEntryType::Filter => "フィルタプラグイン",
            CatalogEntryType::Common => "汎用プラグイン",
            CatalogEntryType::Modification => "MOD",
            CatalogEntryType::Script => "スクリプト",
            CatalogEntryType::Other => "その他",
            CatalogEntryType::Custom(custom) => custom.as_str(),
        };
        serializer.serialize_str(s)
    }
}

/* ---------- root ---------- */

pub type CatalogIndex = Vec<CatalogEntry>;
