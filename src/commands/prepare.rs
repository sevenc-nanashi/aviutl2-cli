use anyhow::{Context, Result, bail};
use fs_err as fs;
use fs_err::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use crate::config::{PlacementMethod, load_config};
use crate::util::{
    copy_to_destination, create_symlink, development_dir, extract_zip, find_aviutl2_data_dir,
};

const API_BASE: &str = "https://api.aviutl2.jp";

pub fn aviutl2() -> Result<()> {
    let config = load_config()?;
    let dev = config
        .development
        .as_ref()
        .context("development 設定が必要です")?;
    let install_dir = development_dir(dev)?;
    fs::create_dir_all(&install_dir)
        .with_context(|| format!("ディレクトリ作成に失敗しました: {}", install_dir.display()))?;
    if let Ok(current_version) = fs::read_to_string(install_dir.join(".aviutl2-version"))
        && current_version == dev.aviutl2_version
    {
        log::info!(
            "AviUtl2 のバージョンが一致しています: {}",
            dev.aviutl2_version
        );
        return Ok(());
    }

    let zip_path = download_aviutl2_zip(&dev.aviutl2_version)?;
    extract_zip(&zip_path, &install_dir)?;
    fs::remove_file(&zip_path).ok();
    log::info!("AviUtl2 を展開しました: {}", install_dir.display());
    let mut version = File::create(install_dir.join(".aviutl2-version"))?;
    version.write_all(dev.aviutl2_version.as_bytes())?;
    Ok(())
}

pub fn artifacts(force: bool, profile: Option<String>, refresh: bool) -> Result<()> {
    let config = load_config()?;
    let dev = config
        .development
        .as_ref()
        .context("development 設定が必要です")?;
    let install_dir = development_dir(dev)?;
    let profile = profile
        .as_deref()
        .or(dev.profile.as_deref())
        .unwrap_or("debug");
    let artifacts = super::develop::resolve_artifacts(&config, Some(profile), None, refresh)?;
    let data_dir = find_aviutl2_data_dir(&install_dir)?;

    for artifact in artifacts {
        let source = artifact.source;
        let dest = data_dir.join(&artifact.destination);
        match artifact.placement_method {
            PlacementMethod::Symlink => {
                let relative_source = if source.is_absolute() {
                    source.clone()
                } else {
                    std::env::current_dir()
                        .with_context(|| "カレントディレクトリの取得に失敗しました")?
                        .join(&source)
                };
                let to_source_relative =
                    pathdiff::diff_paths(&relative_source, dest.parent().unwrap()).with_context(
                        || {
                            format!(
                                "シンボリックリンクの相対パス計算に失敗しました: {} -> {}",
                                dest.display(),
                                relative_source.display()
                            )
                        },
                    )?;
                create_symlink(&to_source_relative, &dest, force)?
            }
            _ => {
                if !source.exists() {
                    log::warn!("source が見つかりません: {}", source.display());
                    continue;
                }
                copy_to_destination(&source, &dest, force)?
            }
        }
    }
    log::info!("成果物のシンボリックリンクを作成しました");
    Ok(())
}

fn download_aviutl2_zip(version: &str) -> Result<PathBuf> {
    let mut url = String::from(API_BASE);
    url.push_str("/download");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .max_redirects(5)
        .build()
        .into();
    let response = agent
        .get(&url)
        .query("version", version)
        .query("type", "zip")
        .header("User-Agent", "aviutl2-cli")
        .call()
        .with_context(|| "AviUtl2 のダウンロードに失敗しました")?;
    let status = response.status();
    if !status.is_success() {
        bail!("AviUtl2 のダウンロードに失敗しました: {}", status);
    }

    let (_parts, body) = response.into_parts();
    let mut reader = body.into_reader();
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;

    let file_name = format!("aviutl2-{}.zip", version.replace('/', "_"));
    let mut tmp_path = std::env::temp_dir();
    tmp_path.push(file_name);
    let mut file = File::create(&tmp_path)?;
    file.write_all(&buf)?;
    Ok(tmp_path)
}
