use anyhow::{Context, Result};
use fs_err as fs;
use std::path::PathBuf;

use crate::config::Config;
use crate::config::load_config;
use crate::util::{copy_to_destination, create_zip, fill_template, release_stage_dir};

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
    super::develop::run_optional_commands(release.prebuild.as_ref())?;
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
    super::develop::run_optional_commands(release.postbuild.as_ref())?;
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

    for artifact in artifacts {
        super::develop::run_build_commands(&artifact.build_commands)?;
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
