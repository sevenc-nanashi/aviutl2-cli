use anyhow::{Context, Result, bail};
use fs_err as fs;
use fs_err::File;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use crate::config::{Artifact, ConfigLoadOpts, PlacementMethod, load_config};
use crate::util::{
    check_and_warn_symlink_capability, copy_to_destination, create_symlink, development_dir,
    extract_zip, find_aviutl2_data_dir, prepare_snapshot_path, remove_path,
};

const API_BASE: &str = "https://api.aviutl2.jp";

pub fn aviutl2(opts: &ConfigLoadOpts) -> Result<()> {
    let config = load_config(opts)?;
    let dev = config
        .development
        .as_ref()
        .context("development 設定が必要です")?;
    let install_dir = development_dir(dev)?;
    aviutl2_in(&install_dir, &dev.aviutl2_version)?;
    init_config(&install_dir)?;
    Ok(())
}

pub fn aviutl2_in(install_dir: &std::path::Path, aviutl2_version: &str) -> Result<()> {
    fs::create_dir_all(install_dir)
        .with_context(|| format!("ディレクトリ作成に失敗しました: {}", install_dir.display()))?;
    let aviutl2_version = resolve_version(aviutl2_version)?;
    if let Ok(current_version) = fs::read_to_string(install_dir.join(".aviutl2-version"))
        && current_version == aviutl2_version
    {
        tracing::info!("AviUtl2 のバージョンが一致しています: {}", aviutl2_version);
        return Ok(());
    }

    let zip_path = download_aviutl2_zip(&aviutl2_version)?;
    extract_zip(&zip_path, install_dir)?;
    fs::remove_file(&zip_path).ok();
    tracing::info!("AviUtl2 を展開しました: {}", install_dir.display());
    let mut version = File::create(install_dir.join(".aviutl2-version"))?;
    version.write_all(aviutl2_version.as_bytes())?;
    Ok(())
}

fn init_config(install_dir: &std::path::Path) -> Result<()> {
    static CONFIG: &str = dedent::dedent!(
        r#"
        [Window.log]
        hide=0
        [Logger]
        ViewLogLevel=1
        FileLogLevel=1
        "#
    );
    let data_dir = install_dir.join("data");
    let config_path = data_dir.join("aviutl2.ini");
    if config_path.exists() {
        tracing::info!(
            "既に aviutl2.ini が存在するため、初期設定の書き込みをスキップします: {}",
            config_path.display()
        );
        return Ok(());
    }
    fs::create_dir_all(config_path.parent().unwrap()).with_context(|| {
        format!(
            "ディレクトリ作成に失敗しました: {}",
            config_path.parent().unwrap().display()
        )
    })?;
    fs::write(&config_path, CONFIG).with_context(|| {
        format!(
            "初期設定の書き込みに失敗しました: {}",
            config_path.display()
        )
    })?;
    tracing::info!("初期設定を書き込みました: {}", config_path.display());
    Ok(())
}

pub fn artifacts(
    force: bool,
    profile: Option<String>,
    refresh: bool,
    opts: &ConfigLoadOpts,
) -> Result<()> {
    let config = load_config(opts)?;
    let dev = config
        .development
        .as_ref()
        .context("development 設定が必要です")?;
    let install_dir = development_dir(dev)?;
    let profile = profile.as_deref().unwrap_or(&dev.profile);
    let artifacts = super::develop::resolve_artifacts(&config, Some(profile), None, refresh)?;
    let data_dir = find_aviutl2_data_dir(&install_dir)?;

    let has_symlink_artifact = artifacts
        .iter()
        .any(|a| matches!(a.placement_method, PlacementMethod::Symlink));
    if has_symlink_artifact {
        check_and_warn_symlink_capability()?;
    }

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
                    tracing::warn!("source が見つかりません: {}", source.display());
                    continue;
                }
                copy_to_destination(&source, &dest, force)?
            }
        }
    }

    let catalog = crate::catalog::load_catalog_index(refresh)?;
    crate::catalog::sync(&data_dir, &catalog, &dev.catalog_dependencies)?;

    tracing::info!("成果物のシンボリックリンクを作成しました");
    save_prepare_snapshot(&config.artifacts, &dev.aviutl2_version)?;
    Ok(())
}

pub fn cleanup_data_generated_by_prepare(opts: &ConfigLoadOpts) -> Result<()> {
    let config = load_config(opts)?;
    let dev = config
        .development
        .as_ref()
        .context("development 設定が必要です")?;
    let install_dir = development_dir(dev)?;
    let data_dir = match find_aviutl2_data_dir(&install_dir) {
        Ok(data_dir) => data_dir,
        Err(err) => {
            tracing::warn!(
                "AviUtl2 の data ディレクトリが見つからないため、prepare の事前削除をスキップします: {err}"
            );
            return Ok(());
        }
    };

    let mut destinations = BTreeSet::new();
    if let Some(snapshot) = load_prepare_snapshot()? {
        for artifact in snapshot.artifacts.into_values() {
            destinations.insert(PathBuf::from(artifact.destination));
        }
    }
    for artifact in config.artifacts.into_values() {
        destinations.insert(PathBuf::from(artifact.destination));
    }

    for destination in destinations {
        let Some(target) = resolve_data_destination(&data_dir, &destination) else {
            tracing::warn!(
                "data 配下でないため削除をスキップします: {}",
                destination.display()
            );
            continue;
        };
        if target.exists() || target.is_symlink() {
            remove_path(&target)?;
            tracing::info!("前回の成果物を削除しました: {}", target.display());
        }
    }

    Ok(())
}

fn resolve_data_destination(data_dir: &Path, destination: &Path) -> Option<PathBuf> {
    if destination.is_absolute() {
        return None;
    }
    let mut normalized = PathBuf::new();
    for component in destination.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            _ => return None,
        }
    }
    if normalized.as_os_str().is_empty() {
        return None;
    }
    Some(data_dir.join(normalized))
}

#[derive(Serialize, Deserialize)]
pub struct PrepareSnapshot {
    pub aviutl2_version: String,
    pub artifacts: BTreeMap<String, Artifact>,
}

pub fn save_prepare_snapshot(
    artifacts: &std::collections::HashMap<String, Artifact>,
    aviutl2_version: &str,
) -> Result<()> {
    let mut ordered = BTreeMap::new();
    for (name, artifact) in artifacts {
        ordered.insert(name.clone(), artifact.clone());
    }
    let snapshot = PrepareSnapshot {
        aviutl2_version: aviutl2_version.to_string(),
        artifacts: ordered,
    };
    let snapshot_path = prepare_snapshot_path()?;
    if let Some(parent) = snapshot_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(&snapshot)?;
    fs::write(&snapshot_path, content)?;
    Ok(())
}

pub fn load_prepare_snapshot() -> Result<Option<PrepareSnapshot>> {
    let snapshot_path = prepare_snapshot_path()?;
    if !snapshot_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(snapshot_path)?;
    let snapshot = serde_json::from_str(&content)?;
    Ok(Some(snapshot))
}

fn resolve_version(version: &str) -> Result<String> {
    #[derive(serde::Deserialize)]
    struct VersionResponse {
        version: String,
    }

    let mut url = String::from(API_BASE);
    url.push_str(&format!("/versions/{}", version));
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .max_redirects(5)
        .build()
        .into();
    let mut response = agent
        .get(&url)
        .header("User-Agent", "aviutl2-cli")
        .call()
        .with_context(|| format!("AviUtl2 のバージョン情報の取得に失敗しました: {}", version))?;
    let status = response.status();
    if !status.is_success() {
        bail!(
            "AviUtl2 のバージョン情報の取得に失敗しました: {} (HTTP {})",
            version,
            status
        );
    }

    let version_info: VersionResponse = response
        .body_mut()
        .read_json()
        .with_context(|| "AviUtl2 のバージョン情報の解析に失敗しました")?;

    Ok(version_info.version)
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

#[cfg(test)]
mod tests {
    use super::resolve_data_destination;
    use std::path::{Path, PathBuf};

    #[test]
    fn resolve_data_destination_rejects_escape_path() {
        let data = Path::new("C:/work/data");
        let dest = Path::new("../outside/file.txt");
        assert!(resolve_data_destination(data, dest).is_none());
    }

    #[test]
    fn resolve_data_destination_joins_normal_path() {
        let data = Path::new("C:/work/data");
        let dest = Path::new("Plugin/test.aux2");
        let resolved = resolve_data_destination(data, dest).unwrap();
        assert_eq!(resolved, PathBuf::from("C:/work/data/Plugin/test.aux2"));
    }
}
