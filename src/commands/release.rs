use anyhow::{Context, Result};
use fs_err as fs;
use std::path::PathBuf;

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
    let include = release.include.as_deref();
    let artifacts = super::develop::resolve_artifacts(&config, Some(&profile), include, false)?;
    let output_dir = PathBuf::from(release.output_dir.as_deref().unwrap_or("release"));
    fs::create_dir_all(&output_dir)?;

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

    if let Some(template) = release.package_template.as_ref() {
        let template_path = PathBuf::from(template);
        let target = stage_dir.join("package.txt");
        let content = fs::read_to_string(&template_path).with_context(|| {
            format!(
                "package.txt の読み込みに失敗しました: {}",
                template_path.display()
            )
        })?;
        let content = fill_template(&content, &config.project);
        let content = normalize_to_crlf(&content);
        fs::write(&target, content).with_context(|| {
            format!("package.txt の書き込みに失敗しました: {}", target.display())
        })?;
    }

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
    Ok(())
}

fn normalize_to_crlf(input: &str) -> String {
    let normalized = input.replace("\r\n", "\n");
    normalized.replace('\n', "\r\n")
}
