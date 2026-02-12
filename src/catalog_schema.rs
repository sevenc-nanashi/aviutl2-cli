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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CatalogEntryType {
    #[serde(rename = "本体")]
    AviUtl2,
    #[serde(rename = "出力プラグイン")]
    Output,
    #[serde(rename = "入力プラグイン")]
    Input,
    #[serde(rename = "フィルタプラグイン")]
    Filter,
    #[serde(rename = "汎用プラグイン")]
    Common,
    #[serde(rename = "MOD")]
    Modification,
    #[serde(rename = "スクリプト")]
    Script,
}

/* ---------- root ---------- */

pub type CatalogIndex = Vec<CatalogEntry>;
