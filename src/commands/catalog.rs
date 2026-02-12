use anyhow::{Context, Result};
use fs_err as fs;
use std::path::Path;
use time::OffsetDateTime;
use walkdir::WalkDir;
use xxhash_rust::xxh3::xxh3_128;

use crate::catalog_schema::{
    CatalogEntry, CatalogEntryType, CatalogIndex, Copyright, GithubSource, GoogleDriveSource,
    Image, Installer, InstallerAction, InstallerSource, License, Version, VersionFile,
};
use crate::config::{
    CC0LicenseType, Catalog, CatalogAction, CatalogDescription, CatalogDownloadSource,
    CatalogLicense, CatalogType, CustomCatalogLicenseType, OtherCatalogLicenseType,
    TemplateCatalogLicenseType, UnknownCatalogLicenseType, load_config,
};
pub fn run() -> Result<()> {
    let config = load_config()?;
    let catalog = config.catalog.as_ref().context("catalog 設定が必要です")?;
    let versions = build_versions(&config)?;
    let generated_pattern = generate_au2pkg_pattern(
        &config.project,
        config
            .release
            .as_ref()
            .and_then(|release| release.zip_name.as_deref()),
    );
    let output = build_catalog_index(catalog, &versions, &generated_pattern);
    let content = serde_json::to_string_pretty(&output)?;

    let target_dir = std::env::current_dir()
        .context("カレントディレクトリの取得に失敗しました")?
        .join(config.release.map_or("release".to_string(), |r| {
            r.output_dir
                .clone()
                .unwrap_or_else(|| "release".to_string())
        }));
    fs::create_dir_all(&target_dir)
        .with_context(|| format!("ディレクトリ作成に失敗しました: {}", target_dir.display()))?;
    let target = target_dir.join("catalog.json");
    fs::write(&target, content).with_context(|| {
        format!(
            "catalog.json の書き込みに失敗しました: {}",
            target.display()
        )
    })?;
    log::info!("catalog.json を出力しました: {}", target.display());
    Ok(())
}

fn build_catalog_index(
    catalog: &Catalog,
    versions: &[Version],
    generated_pattern: &str,
) -> CatalogIndex {
    let generated_install_steps = versions
        .first()
        .map(|version| default_install_steps(&version.file))
        .unwrap_or_default();
    vec![CatalogEntry {
        id: catalog.id.clone(),
        name: catalog.name.clone(),
        entry_type: map_catalog_type(&catalog.catalog_type),
        summary: catalog.summary.clone(),
        description: map_description(&catalog.description),
        author: catalog.author.clone(),
        repo_url: catalog.homepage.clone(),
        licenses: vec![map_license(&catalog.license)],
        niconi_commons_id: catalog.niconi_commons_id.clone(),
        tags: catalog.tags.clone().unwrap_or_default(),
        dependencies: catalog.dependencies.clone().unwrap_or_default(),
        images: Vec::<Image>::new(),
        installer: Installer {
            source: map_source(&catalog.download_source, generated_pattern),
            install: catalog
                .install_steps
                .as_ref()
                .map(|steps| steps.iter().map(map_action).collect())
                .unwrap_or(generated_install_steps),
            uninstall: catalog
                .uninstall_steps
                .as_ref()
                .map(|steps| steps.iter().map(map_action).collect())
                .unwrap_or_default(),
        },
        version: versions.to_vec(),
    }]
}

fn build_versions(config: &crate::config::Config) -> Result<Vec<Version>> {
    let release = config.release.as_ref();
    let profile = release
        .and_then(|value| value.profile.as_deref())
        .unwrap_or("release");
    let include = release.and_then(|value| value.include.as_deref());
    let package_template = release.and_then(|value| value.package_template.as_deref());
    let stage_dir =
        super::release::build_release_stage(config, profile, include, package_template, false)?;
    let files = collect_version_files(&stage_dir)?;
    let release_date = OffsetDateTime::now_utc()
        .format(&time::format_description::parse("[year]-[month]-[day]")?)
        .unwrap_or_default();
    Ok(vec![Version {
        version: config.project.version.clone(),
        release_date,
        file: files,
    }])
}

fn collect_version_files(stage_dir: &Path) -> Result<Vec<VersionFile>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(stage_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        let path = entry.path();
        if !entry.file_type().is_file() {
            continue;
        }
        let relative_path = path
            .strip_prefix(stage_dir)
            .with_context(|| format!("相対パスの生成に失敗しました: {}", path.display()))?;
        let bytes = fs::read(path)
            .with_context(|| format!("成果物の読み込みに失敗しました: {}", path.display()))?;
        files.push(VersionFile {
            path: to_slash_path(relative_path),
            xxh3_128: format!("{:032x}", xxh3_128(&bytes)),
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn default_install_steps(files: &[VersionFile]) -> Vec<InstallerAction> {
    let mut actions = vec![InstallerAction::Download {}, InstallerAction::Extract {}];
    for file in files {
        if file.path.eq_ignore_ascii_case("package.txt") {
            continue;
        }
        actions.push(InstallerAction::Copy {
            from: file.path.clone(),
            to: file.path.clone(),
        });
    }
    actions
}

fn map_catalog_type(catalog_type: &CatalogType) -> CatalogEntryType {
    match catalog_type {
        CatalogType::Output => CatalogEntryType::Output,
        CatalogType::Input => CatalogEntryType::Input,
        CatalogType::Filter => CatalogEntryType::Filter,
        CatalogType::Common => CatalogEntryType::Common,
        CatalogType::Modification => CatalogEntryType::Modification,
        CatalogType::Script => CatalogEntryType::Script,
        CatalogType::Language => CatalogEntryType::Script,
    }
}

fn to_slash_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn map_description(description: &CatalogDescription) -> String {
    match description {
        CatalogDescription::Plain(value) => value.clone(),
        CatalogDescription::Url(value) => value.url.clone(),
        CatalogDescription::Inline(value) => value.content.clone(),
    }
}

fn map_license(license: &CatalogLicense) -> License {
    match license {
        CatalogLicense::Template(template) => License {
            license_type: match template.license_type {
                TemplateCatalogLicenseType::Mit => "MIT",
                TemplateCatalogLicenseType::Apache20 => "Apache-2.0",
                TemplateCatalogLicenseType::Bsd2Clause => "BSD-2-Clause",
                TemplateCatalogLicenseType::Bsd3Clause => "BSD-3-Clause",
            }
            .to_string(),
            is_custom: false,
            copyrights: vec![Copyright {
                years: template.year.clone(),
                holder: template.author.clone(),
            }],
            license_body: None,
        },
        CatalogLicense::Custom(custom) => License {
            license_type: match custom.license_type {
                CustomCatalogLicenseType::Mit => "mit",
                CustomCatalogLicenseType::Apache20 => "apache-2.0",
                CustomCatalogLicenseType::Bsd2Clause => "bsd-2-clause",
                CustomCatalogLicenseType::Bsd3Clause => "bsd-3-clause",
            }
            .to_string(),
            is_custom: true,
            copyrights: vec![],
            license_body: Some(custom.text.clone()),
        },
        CatalogLicense::Cc0(cc0) => License {
            license_type: match cc0.license_type {
                CC0LicenseType::Cc0 => "cc0",
            }
            .to_string(),
            is_custom: false,
            copyrights: vec![],
            license_body: None,
        },
        CatalogLicense::Other(other) => License {
            license_type: match other.license_type {
                OtherCatalogLicenseType::Other => other.name.as_deref().unwrap_or("other"),
            }
            .to_string(),
            is_custom: true,
            copyrights: vec![],
            license_body: Some(other.text.clone()),
        },
        CatalogLicense::Unknown(unknown) => License {
            license_type: match unknown.license_type {
                UnknownCatalogLicenseType::Unknown => "unknown",
            }
            .to_string(),
            is_custom: false,
            copyrights: vec![],
            license_body: None,
        },
    }
}

fn map_source(source: &CatalogDownloadSource, generated_pattern: &str) -> InstallerSource {
    match source {
        CatalogDownloadSource::Direct { url } => InstallerSource::Direct {
            direct: url.clone(),
        },
        CatalogDownloadSource::Booth { url } => InstallerSource::Booth { booth: url.clone() },
        CatalogDownloadSource::Github {
            owner,
            repo,
            pattern,
        } => InstallerSource::Github {
            github: GithubSource {
                owner: owner.clone(),
                repo: repo.clone(),
                pattern: pattern
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| generated_pattern.to_string()),
            },
        },
        CatalogDownloadSource::GoogleDrive { id } => InstallerSource::GoogleDrive {
            google_drive: GoogleDriveSource { id: id.clone() },
        },
    }
}

fn generate_au2pkg_pattern(project: &crate::config::Project, zip_base: Option<&str>) -> String {
    let zip_base = zip_base.unwrap_or("{name}-v{version}");
    let zip_name_template = if zip_base.ends_with(".au2pkg.zip") {
        zip_base.to_string()
    } else {
        format!("{zip_base}.au2pkg.zip")
    };

    let name_token = "__AU2_NAME_TOKEN__";
    let version_token = "__AU2_VERSION_TOKEN__";
    let tokenized = zip_name_template
        .replace("{name}", name_token)
        .replace("{version}", version_token);
    let mut escaped = regex_escape(&tokenized);
    escaped = escaped.replace(name_token, &regex_escape(&project.name));
    escaped = escaped.replace(version_token, "[^/]+");
    format!("^{escaped}$")
}

fn regex_escape(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '^' | '$' | '.' | '|' | '?' | '*' | '+' | '(' | ')' | '[' | ']' | '{' | '}' => {
                output.push('\\');
                output.push(ch);
            }
            _ => output.push(ch),
        }
    }
    output
}

fn map_action(action: &CatalogAction) -> InstallerAction {
    match action {
        CatalogAction::Download => InstallerAction::Download {},
        CatalogAction::Extract => InstallerAction::Extract {},
        CatalogAction::Copy { from, to } => InstallerAction::Copy {
            from: from.clone(),
            to: to.clone(),
        },
        CatalogAction::Delete { path } => InstallerAction::Delete { path: path.clone() },
        CatalogAction::Run {
            path,
            args,
            elevate,
        } => InstallerAction::Run {
            path: path.clone(),
            args: args.clone(),
            elevate: *elevate,
        },
    }
}
