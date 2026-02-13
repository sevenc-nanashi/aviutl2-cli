use anyhow::{Context, Result};
use fs_err as fs;
use std::path::PathBuf;

use crate::{
    catalog_schema,
    config::{self, Config, load_config},
    util::{copy_to_destination, create_zip, fill_template, release_stage_dir},
};

pub fn run(profile: Option<String>, set_version: Option<String>) -> Result<()> {
    let mut config = load_config()?;
    if let Some(version) = set_version {
        config.project.version = version;
    }
    let release = config.release.as_ref().context("release 設定が必要です")?;
    let profile = profile
        .or_else(|| release.profile.clone())
        .unwrap_or_else(|| "release".to_string());
    let output_dir = PathBuf::from(release.output_dir.as_deref().unwrap_or("release"));
    fs::create_dir_all(&output_dir)?;
    super::develop::run_optional_commands(release.prebuild.as_ref(), config.build_group.as_ref())?;
    let stage_dir = build_release_stage(
        &config,
        &profile,
        release.include.as_deref(),
        release.package_template.as_deref(),
        false,
    )?;

    let zip_base = release
        .zip_name
        .clone()
        .unwrap_or_else(|| "{name}-v{version}".to_string());
    let zip_name = fill_template(&zip_base, &config.project);
    let zip_file_name = if zip_name.ends_with(".au2pkg.zip") {
        zip_name
    } else {
        format!("{zip_name}.au2pkg.zip")
    };
    let zip_path = output_dir.join(zip_file_name);
    create_zip(&stage_dir, &zip_path)?;
    log::info!("リリースパッケージを作成しました: {}", zip_path.display());
    super::develop::run_optional_commands(release.postbuild.as_ref(), config.build_group.as_ref())?;

    if let Some(catalog_config) = &config.catalog {
        log::warn!("カタログ生成機能は実験的機能です。将来のバージョンで変更または削除される可能性があります。");
        let versions = build_versions(&config, &stage_dir)?;
        let generated_pattern =
            generate_au2pkg_pattern(&config.project, release.zip_name.as_deref());
        let catalog_index =
            build_catalog_index(catalog_config, &stage_dir, &versions, &generated_pattern)?;
        let catalog_json = serde_json::to_string_pretty(&catalog_index)
            .context("カタログ JSON の生成に失敗しました")?;
        let catalog_path = output_dir.join("catalog.json");
        fs::write(&catalog_path, catalog_json).with_context(|| {
            format!(
                "カタログ JSON の書き込みに失敗しました: {}",
                catalog_path.display()
            )
        })?;
        log::info!("カタログ JSON を作成しました: {}", catalog_path.display());
    }
    Ok(())
}

pub(crate) fn build_release_stage(
    config: &Config,
    profile: &str,
    include: Option<&[String]>,
    package_template: Option<&str>,
    refresh: bool,
) -> Result<PathBuf> {
    let artifacts = super::develop::resolve_artifacts(config, Some(profile), include, refresh)?;
    build_release_stage_from_artifacts(artifacts, package_template, &config.project)
}

pub(crate) fn build_release_stage_from_artifacts(
    artifacts: Vec<super::develop::ResolvedArtifact>,
    package_template: Option<&str>,
    project: &crate::config::Project,
) -> Result<PathBuf> {
    let stage_dir = release_stage_dir()?;
    if stage_dir.exists() {
        fs::remove_dir_all(&stage_dir)?;
    }
    fs::create_dir_all(&stage_dir)?;

    let mut executed_groups = std::collections::HashSet::new();
    for artifact in artifacts {
        super::develop::run_build_plan(&artifact.build_plan, &mut executed_groups)?;
        copy_to_destination(
            &artifact.source,
            &stage_dir.join(&artifact.destination),
            true,
        )?;
    }

    if let Some(template) = package_template {
        let template_path = PathBuf::from(template);
        let target = stage_dir.join("package.txt");
        let content = fs::read_to_string(&template_path).with_context(|| {
            format!(
                "package.txt の読み込みに失敗しました: {}",
                template_path.display()
            )
        })?;
        let content = fill_template(&content, project);
        let content = normalize_to_crlf(&content);
        fs::write(&target, content).with_context(|| {
            format!("package.txt の書き込みに失敗しました: {}", target.display())
        })?;
    }
    Ok(stage_dir)
}

fn normalize_to_crlf(input: &str) -> String {
    let normalized = input.replace("\r\n", "\n");
    normalized.replace('\n', "\r\n")
}

fn build_catalog_index(
    catalog: &config::Catalog,
    stage_dir: &std::path::Path,
    versions: &[catalog_schema::Version],
    generated_pattern: &str,
) -> Result<catalog_schema::CatalogIndex> {
    let install_steps = if let Some(steps) = &catalog.install_steps {
        steps.iter().map(map_action).collect::<Vec<_>>()
    } else {
        default_install_steps(stage_dir)?
    };
    let uninstall_steps = if let Some(steps) = &catalog.uninstall_steps {
        steps.iter().map(map_action).collect::<Vec<_>>()
    } else {
        default_uninstall_steps(&install_steps)?
    };
    Ok(vec![catalog_schema::CatalogEntry {
        id: catalog.id.clone(),
        name: catalog.name.clone(),
        entry_type: map_catalog_type(&catalog.catalog_type),
        summary: catalog.summary.clone(),
        description: map_description(&catalog.description),
        author: catalog.author.clone(),
        original_author: catalog.original_author.clone(),
        repo_url: catalog.homepage.clone(),
        licenses: vec![map_license(&catalog.license)?],
        niconi_commons_id: catalog.niconi_commons_id.clone(),
        tags: catalog.tags.clone().unwrap_or_default(),
        dependencies: catalog.dependencies.clone().unwrap_or_default(),
        images: Vec::<catalog_schema::Image>::new(),
        installer: catalog_schema::Installer {
            source: map_source(&catalog.download_source, generated_pattern),
            install: install_steps,
            uninstall: uninstall_steps,
        },
        version: versions.to_vec(),
    }])
}

fn build_versions(
    config: &crate::config::Config,
    stage_dir: &std::path::Path,
) -> Result<Vec<catalog_schema::Version>> {
    let files = collect_version_files(stage_dir)?;
    let release_date = time::OffsetDateTime::now_utc()
        .format(&time::format_description::parse("[year]-[month]-[day]")?)
        .unwrap_or_default();
    Ok(vec![catalog_schema::Version {
        version: config.project.version.clone(),
        release_date,
        file: files,
    }])
}

fn collect_version_files(stage_dir: &std::path::Path) -> Result<Vec<catalog_schema::VersionFile>> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(stage_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        let path = entry.path();
        if !entry.file_type().is_file() {
            continue;
        }
        if path.file_name().and_then(|n| n.to_str()) == Some("package.txt") {
            continue;
        }
        let relative_path = path
            .strip_prefix(stage_dir)
            .with_context(|| format!("相対パスの生成に失敗しました: {}", path.display()))?;
        let bytes = fs::read(path)
            .with_context(|| format!("成果物の読み込みに失敗しました: {}", path.display()))?;
        files.push(catalog_schema::VersionFile {
            path: to_catalog_relative_path(relative_path),
            xxh3_128: format!("{:032x}", xxhash_rust::xxh3::xxh3_128(&bytes)),
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn default_install_steps(
    stage_dir: &std::path::Path,
) -> Result<Vec<catalog_schema::InstallerAction>> {
    let mut actions = vec![
        catalog_schema::InstallerAction::Download {},
        catalog_schema::InstallerAction::Extract {},
    ];

    for entry in walkdir::WalkDir::new(stage_dir) {
        let entry = entry.with_context(|| "ディレクトリの走査に失敗しました")?;
        let file = entry.path();
        if !entry.file_type().is_file() {
            continue;
        }
        let relative_path = file
            .strip_prefix(stage_dir)
            .with_context(|| format!("相対パスの生成に失敗しました: {}", file.display()))?;
        if relative_path == "package.txt" {
            continue;
        }
        actions.push(catalog_schema::InstallerAction::Copy {
            from: format!("{{tmp}}/{}", normalize_path_separator(relative_path)),
            to: to_catalog_relative_path(relative_path.parent().unwrap()),
        });
    }
    Ok(actions)
}

fn default_uninstall_steps(
    install_steps: &[catalog_schema::InstallerAction],
) -> Result<Vec<catalog_schema::InstallerAction>> {
    let mut actions = Vec::new();
    for action in install_steps.iter().rev() {
        if let catalog_schema::InstallerAction::Copy { to, from, .. } = action {
            let file_path = format!("{}/{}", to, from.split('/').next_back().unwrap());
            actions.push(catalog_schema::InstallerAction::Delete { path: file_path });
        }
    }
    Ok(actions)
}

fn to_catalog_relative_path(path: &std::path::Path) -> String {
    let normalized = normalize_path_separator(path);
    let trimmed = normalized.strip_prefix("./").unwrap_or(&normalized);
    let lowered = trimmed.to_lowercase();
    if lowered == "plugin" {
        "{pluginsDir}".to_string()
    } else if lowered == "script" {
        "{scriptsDir}".to_string()
    } else if lowered.starts_with("plugin/") {
        format!("{{pluginsDir}}/{}", &trimmed["plugin/".len()..])
    } else if lowered.starts_with("script/") {
        format!("{{scriptsDir}}/{}", &trimmed["script/".len()..])
    } else {
        format!("{{dataDir}}/{}", trimmed)
    }
}
fn normalize_path_separator(path: &std::path::Path) -> String {
    let path = path.to_string_lossy();
    let replaced = path.replace("\\", "/");
    replaced.strip_prefix("./").unwrap_or(&replaced).to_string()
}

fn map_catalog_type(catalog_type: &config::CatalogType) -> catalog_schema::CatalogEntryType {
    match catalog_type {
        config::CatalogType::Output => catalog_schema::CatalogEntryType::Output,
        config::CatalogType::Input => catalog_schema::CatalogEntryType::Input,
        config::CatalogType::Filter => catalog_schema::CatalogEntryType::Filter,
        config::CatalogType::Common => catalog_schema::CatalogEntryType::Common,
        config::CatalogType::Modification => catalog_schema::CatalogEntryType::Modification,
        config::CatalogType::Script => catalog_schema::CatalogEntryType::Script,
        config::CatalogType::Language => catalog_schema::CatalogEntryType::Script,
    }
}

fn map_description(description: &config::CatalogDescription) -> String {
    match description {
        config::CatalogDescription::Plain(value) => value.clone(),
        config::CatalogDescription::Url(value) => value.url.clone(),
        config::CatalogDescription::Inline(value) => value.content.clone(),
    }
}

fn map_license(license: &config::CatalogLicense) -> Result<catalog_schema::License> {
    Ok(match license {
        config::CatalogLicense::Template(template) => catalog_schema::License {
            license_type: match template.license_type {
                config::TemplateCatalogLicenseType::Mit => "MIT",
                config::TemplateCatalogLicenseType::Apache20 => "Apache-2.0",
                config::TemplateCatalogLicenseType::Bsd2Clause => "BSD-2-Clause",
                config::TemplateCatalogLicenseType::Bsd3Clause => "BSD-3-Clause",
            }
            .to_string(),
            is_custom: false,
            copyrights: vec![catalog_schema::Copyright {
                years: template.year.clone(),
                holder: template.author.clone(),
            }],
            license_body: None,
        },
        config::CatalogLicense::Custom(custom) => catalog_schema::License {
            license_type: match custom.license_type {
                config::TemplateCatalogLicenseType::Mit => "MIT",
                config::TemplateCatalogLicenseType::Apache20 => "Apache-2.0",
                config::TemplateCatalogLicenseType::Bsd2Clause => "BSD-2-Clause",
                config::TemplateCatalogLicenseType::Bsd3Clause => "BSD-3-Clause",
            }
            .to_string(),
            is_custom: true,
            copyrights: vec![],
            license_body: Some(resolve_license_text(&custom.text)?),
        },
        config::CatalogLicense::Cc0(cc0) => catalog_schema::License {
            license_type: match cc0.license_type {
                config::CC0LicenseType::Cc0 => "CC0-1.0",
            }
            .to_string(),
            is_custom: false,
            copyrights: vec![],
            license_body: None,
        },
        config::CatalogLicense::Other(other) => catalog_schema::License {
            license_type: match other.name.as_deref() {
                Some("custom") | None => "カスタムライセンス".to_string(),
                Some(name) => name.to_string(),
            },
            is_custom: true,
            copyrights: vec![],
            license_body: Some(resolve_license_text(&other.text)?),
        },
        config::CatalogLicense::Unknown(unknown) => catalog_schema::License {
            license_type: match unknown.license_type {
                config::UnknownCatalogLicenseType::Unknown => "不明",
            }
            .to_string(),
            is_custom: false,
            copyrights: vec![],
            license_body: None,
        },
    })
}

fn resolve_license_text(text: &config::CatalogLicenseText) -> Result<String> {
    match text {
        config::CatalogLicenseText::Inline { content } => Ok(content.clone()),
        config::CatalogLicenseText::File { path } => {
            let content = fs::read_to_string(path)
                .with_context(|| format!("ライセンスファイルの読み込みに失敗しました: {}", path))?;
            Ok(content)
        }
    }
}

fn map_source(
    source: &config::CatalogDownloadSource,
    generated_pattern: &str,
) -> catalog_schema::InstallerSource {
    match source {
        config::CatalogDownloadSource::Direct { url } => catalog_schema::InstallerSource::Direct {
            direct: url.clone(),
        },
        config::CatalogDownloadSource::Booth { url } => {
            catalog_schema::InstallerSource::Booth { booth: url.clone() }
        }
        config::CatalogDownloadSource::Github {
            owner,
            repo,
            pattern,
        } => catalog_schema::InstallerSource::Github {
            github: catalog_schema::GithubSource {
                owner: owner.clone(),
                repo: repo.clone(),
                pattern: pattern
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| generated_pattern.to_string()),
            },
        },
        config::CatalogDownloadSource::GoogleDrive { id } => {
            catalog_schema::InstallerSource::GoogleDrive {
                google_drive: catalog_schema::GoogleDriveSource { id: id.clone() },
            }
        }
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

fn map_action(action: &config::CatalogAction) -> catalog_schema::InstallerAction {
    match action {
        config::CatalogAction::Download => catalog_schema::InstallerAction::Download {},
        config::CatalogAction::Extract => catalog_schema::InstallerAction::Extract {},
        config::CatalogAction::Copy { from, to } => catalog_schema::InstallerAction::Copy {
            from: from.clone(),
            to: to.clone(),
        },
        config::CatalogAction::Delete { path } => {
            catalog_schema::InstallerAction::Delete { path: path.clone() }
        }
        config::CatalogAction::Run {
            path,
            args,
            elevate,
        } => catalog_schema::InstallerAction::Run {
            path: path.clone(),
            args: args.clone(),
            elevate: *elevate,
        },
    }
}
