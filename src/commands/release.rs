use anyhow::{Context, Result};
use fs_err as fs;
use std::path::PathBuf;

use crate::{
    catalog_schema,
    config::{self, Config, ConfigLoadOpts, load_config},
    util::{copy_to_destination, create_zip, fill_template, release_stage_dir},
};

pub fn run(
    profile: Option<String>,
    set_version: Option<String>,
    opts: &ConfigLoadOpts,
) -> Result<()> {
    let mut config = load_config(opts)?;
    if let Some(version) = set_version {
        config.project.version = version;
    }
    let profile = profile.as_deref().unwrap_or(&config.release.profile);
    let output_dir = PathBuf::from(&config.release.output_dir);
    fs::create_dir_all(&output_dir)?;

    super::develop::run_optional_commands(Some(&config.release.prebuild), &config.build_group)?;

    let stage_dir =
        build_release_stage(&config, profile, config.release.include.as_deref(), false)?;
    prepare_package_files(&stage_dir, &config.release, &config.project)?;

    let zip_base = config.release.zip_name.clone();
    let zip_name = fill_template(&zip_base, &config.project);
    let zip_file_name = if zip_name.ends_with(".au2pkg.zip") {
        zip_name
    } else {
        format!("{zip_name}.au2pkg.zip")
    };
    let zip_path = output_dir.join(zip_file_name);
    create_zip(&stage_dir, &zip_path)?;
    tracing::info!("リリースパッケージを作成しました: {}", zip_path.display());
    super::develop::run_optional_commands(Some(&config.release.postbuild), &config.build_group)?;

    if let Some(catalog_config) = &config.catalog {
        tracing::warn!(
            "カタログ生成機能は実験的機能です。将来のバージョンで変更または削除される可能性があります。"
        );
        let versions = build_versions(&config, &stage_dir)?;
        let generated_pattern =
            generate_au2pkg_pattern(&config.project, Some(&config.release.zip_name));
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
        tracing::info!("カタログ JSON を作成しました: {}", catalog_path.display());
    }
    Ok(())
}

pub(crate) fn build_release_stage(
    config: &Config,
    profile: &str,
    include: Option<&[String]>,
    refresh: bool,
) -> Result<PathBuf> {
    let artifacts = super::develop::resolve_artifacts(config, Some(profile), include, refresh)?;
    build_release_stage_from_artifacts(artifacts)
}

pub(crate) fn build_release_stage_from_artifacts(
    artifacts: Vec<super::develop::ResolvedArtifact>,
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

    Ok(stage_dir)
}

pub(crate) fn prepare_package_files(
    stage_dir: &std::path::Path,
    release_config: &crate::config::Release,
    project: &crate::config::Project,
) -> Result<()> {
    if let Some(template) = &release_config.package_template.as_deref() {
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

    let package_ini_path = stage_dir.join("package.ini");
    let id = &release_config.package_id;
    let name = &release_config.package_name;
    let information = &release_config.package_information;
    let package_ini = format!(
        dedent::dedent!(
            r#"
            [package]
            id={id}
            name={name}
            information={information}
            uninstallSubFolderFile={uninstall_subfolder_file}
            "#
        ),
        id = fill_template(id, project),
        name = fill_template(name, project),
        information = fill_template(information, project),
        uninstall_subfolder_file = if release_config.uninstall_subfolder_file {
            "1"
        } else {
            "0"
        }
    );
    fs::write(&package_ini_path, package_ini).with_context(|| {
        format!(
            "package.ini の書き込みに失敗しました: {}",
            package_ini_path.display()
        )
    })?;

    Ok(())
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
) -> Result<serde_json::Value> {
    use serde_json::{Map, Value, json};

    let install_steps = default_install_steps(stage_dir)?;
    let uninstall_steps = default_uninstall_steps(&install_steps)?;

    let mut installer = Map::new();
    installer.insert("install".to_string(), serde_json::to_value(&install_steps)?);
    installer.insert(
        "uninstall".to_string(),
        serde_json::to_value(&uninstall_steps)?,
    );
    if let Some(download_repo) = &catalog.download_repo {
        installer.insert(
            "source".to_string(),
            json!({
                "github": {
                    "owner": download_repo.owner.clone(),
                    "repo": download_repo.repo.clone(),
                    "pattern": generated_pattern,
                }
            }),
        );
    }

    let mut entry = Map::new();
    entry.insert("id".to_string(), Value::String(catalog.id.clone()));
    entry.insert("version".to_string(), serde_json::to_value(versions)?);
    entry.insert("installer".to_string(), Value::Object(installer));

    if let Some(description_path) = &catalog.description_path {
        let description = fs::read_to_string(description_path).with_context(|| {
            format!(
                "説明文ファイルの読み込みに失敗しました: {}",
                description_path
            )
        })?;
        entry.insert("description".to_string(), Value::String(description));
    }
    if let Some(license_path) = &catalog.license_path {
        let license_path_str = match license_path {
            config::CatalogLicensePath::Simple(path) => path,
            config::CatalogLicensePath::Detailed(def) => &def.path,
        };
        let license = fs::read_to_string(license_path_str).with_context(|| {
            format!(
                "ライセンスファイルの読み込みに失敗しました: {}",
                license_path_str
            )
        })?;
        let license_type = resolve_license_type(license_path, &license);
        entry.insert(
            "licenses".to_string(),
            json!([{
                "type": license_type,
                "isCustom": true,
                "copyrights": [],
                "licenseBody": license
            }]),
        );
    }

    Ok(Value::Array(vec![Value::Object(entry)]))
}

fn resolve_license_type(
    license_path: &config::CatalogLicensePath,
    license_body: &str,
) -> catalog_schema::LicenseType {
    let specified_license_type = match license_path {
        config::CatalogLicensePath::Simple(_) => None,
        config::CatalogLicensePath::Detailed(def) => Some(def.license_type.clone()),
    };
    let detected_license_type = detect_license_type(license_body);
    if detected_license_type == catalog_schema::LicenseType::Custom
        && specified_license_type.is_none()
    {
        tracing::warn!("ライセンスの種別の自動検出に失敗しました。");
    }
    if let Some(specified) = specified_license_type.as_ref()
        && detected_license_type != catalog_schema::LicenseType::Custom
        && specified != &detected_license_type
    {
        tracing::warn!(
            "指定されたライセンスタイプと検出されたライセンスタイプが一致しません。指定: {:?}, 検出: {:?}",
            specified,
            detected_license_type
        );
    }
    specified_license_type.unwrap_or(detected_license_type)
}

fn detect_license_type(license_body: &str) -> catalog_schema::LicenseType {
    let normalized = normalize_license_text(license_body);

    if normalized.contains("apache license")
        && normalized.contains("version 2.0, january 2004")
        && normalized.contains("apache.org/licenses/license-2.0")
    {
        return catalog_schema::LicenseType::Apache20;
    }

    if normalized.contains("gnu general public license")
        && normalized.contains("version 3, 29 june 2007")
    {
        return catalog_schema::LicenseType::Gpl30;
    }

    if normalized.contains("gnu general public license")
        && normalized.contains("version 2, june 1991")
    {
        return catalog_schema::LicenseType::Gpl20;
    }

    if normalized.contains("cc0 1.0 universal")
        || normalized.contains("creative commons legal code")
            && normalized.contains("cc0 1.0 universal")
    {
        return catalog_schema::LicenseType::Cc010;
    }

    if normalized
        .contains("this is free and unencumbered software released into the public domain.")
    {
        return catalog_schema::LicenseType::Unlicense;
    }

    if normalized
        .contains("permission is hereby granted, free of charge, to any person obtaining a copy")
    {
        return catalog_schema::LicenseType::Mit;
    }

    if normalized.contains("redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:")
    {
        if normalized.contains("neither the name of")
            || normalized.contains("nor the names of its contributors may be used")
        {
            return catalog_schema::LicenseType::Bsd3Clause;
        }
        return catalog_schema::LicenseType::Bsd2Clause;
    }

    catalog_schema::LicenseType::Custom
}

fn normalize_license_text(input: &str) -> String {
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn build_versions(
    config: &crate::config::Config,
    stage_dir: &std::path::Path,
) -> Result<Vec<catalog_schema::Version>> {
    let files = collect_version_files(stage_dir)?;
    let release_date = time::OffsetDateTime::now_utc()
        .format(&time::format_description::parse_borrowed::<3>("[year]-[month]-[day]").unwrap())
        .unwrap_or_default();
    Ok(vec![catalog_schema::Version {
        version: config.project.version.clone(),
        release_date,
        file: files,
    }])
}

static CATALOG_EXCLUDED_FILES: &[&str] = &["package.txt", "package.ini"];
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
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| CATALOG_EXCLUDED_FILES.contains(&name))
        {
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
        if CATALOG_EXCLUDED_FILES.contains(&relative_path.to_str().unwrap_or_default()) {
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

fn generate_au2pkg_pattern(project: &crate::config::Project, zip_base: Option<&str>) -> String {
    let default_zip_name = crate::config::Release::default_zip_name();
    let zip_base = zip_base.unwrap_or(&default_zip_name);
    let zip_name_template = if zip_base.ends_with(".au2pkg.zip") {
        zip_base.to_string()
    } else {
        format!("{zip_base}.au2pkg.zip")
    };

    let id_token = "__AU2_ID_TOKEN__";
    let name_token = "__AU2_NAME_TOKEN__";
    let version_token = "__AU2_VERSION_TOKEN__";
    let tokenized = zip_name_template
        .replace("{id}", id_token)
        .replace("{name}", name_token)
        .replace("{version}", version_token);
    let mut escaped = regex_escape(&tokenized);
    escaped = escaped.replace(id_token, &regex_escape(&project.id));
    escaped = escaped.replace(
        name_token,
        &regex_escape(project.name.as_ref().unwrap_or(&project.id)),
    );
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
