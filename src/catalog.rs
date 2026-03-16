use anyhow::Context;

use crate::{catalog_schema::CatalogEntry, config::CatalogDependency};

pub fn load_catalog_index(
    path: &std::path::Path,
    refresh: bool,
) -> anyhow::Result<Vec<CatalogEntry>> {
    if should_reload_catalog_index(path, refresh) {
        log::info!("カタログを再読み込みします: {}", path.display());
        let maybe_entries = std::fs::read_to_string(path)
            .context("カタログの読み込みに失敗しました")
            .and_then(|content| {
                serde_json::from_str::<Vec<CatalogEntry>>(&content)
                    .context("カタログの解析に失敗しました")
            });
        match maybe_entries {
            Ok(entries) => return Ok(entries),
            Err(e) => {
                log::warn!("カタログの読み込みに失敗しました: {}", e);
            }
        }
    }

    let entries = fetch_catalog_index()?;
    std::fs::write(path, serde_json::to_string(&entries)?)
        .context("カタログの保存に失敗しました")?;
    Ok(entries)
}

fn should_reload_catalog_index(path: &std::path::Path, refresh: bool) -> bool {
    if refresh {
        return true;
    }
    if let Ok(metadata) = std::fs::metadata(path)
        && let Ok(modified) = metadata.modified()
        && let Ok(elapsed) = modified.elapsed()
    {
        return elapsed.as_secs() > 3600; // 1時間以上前に更新された場合は再読み込み
    }
    false
}

fn fetch_catalog_index() -> anyhow::Result<Vec<CatalogEntry>> {
    let url =
        "https://raw.githubusercontent.com/Neosku/aviutl2-catalog-data/refs/heads/main/index.json";
    let response = ureq::get(url)
        .call()
        .context("カタログのダウンロードに失敗しました")?;
    if response.status() != 200 {
        anyhow::bail!(
            "カタログのダウンロードに失敗しました: HTTP {}",
            response.status()
        );
    }
    let content = response.into_body().read_json()?;
    Ok(content)
}

pub fn install(
    data_root: &std::path::Path,
    entry: &CatalogEntry,
    dependency: &CatalogDependency,
) -> anyhow::Result<()> {
    let installer_source = &entry.installer.source;
    let download_url = match installer_source {
        crate::catalog_schema::InstallerSource::Github { github } => {
            resolve_github_downnload_url(&github.owner, &github.repo, &github.pattern)?
        }
        crate::catalog_schema::InstallerSource::GoogleDrive { google_drive } => {
            format!(
                "https://drive.google.com/uc?export=download&id={id}",
                id = google_drive.id
            )
        }
        crate::catalog_schema::InstallerSource::Direct { direct } => direct.clone(),
        crate::catalog_schema::InstallerSource::Booth { booth: _ } => {
            anyhow::bail!("Booth からのインストールはサポートされていません");
        }
    };

    log::info!("ダウンロード URL: {}", download_url);
    let zip_path = data_root.join("downloaded.tmp");
    let response = ureq::get(&download_url)
        .call()
        .context("ファイルのダウンロードに失敗しました")?;
    if response.status() != 200 {
        anyhow::bail!(
            "ファイルのダウンロードに失敗しました: HTTP {}",
            response.status()
        );
    }
    let mut reader = response.into_body().into_reader();
    let mut file =
        std::fs::File::create(&zip_path).context("ダウンロードしたファイルの保存に失敗しました")?;
    std::io::copy(&mut reader, &mut file)
        .context("ダウンロードしたファイルの保存に失敗しました")?;
    log::info!("ファイルを保存しました: {}", zip_path.display());

    anyhow::bail!("インストーラーの実行はまだ実装されていません");
}

fn resolve_github_downnload_url(owner: &str, repo: &str, pattern: &str) -> anyhow::Result<String> {
    #[derive(Debug, serde::Deserialize)]
    struct MinimumGithubRelease {
        assets: Vec<MinimumGithubAsset>,
    }
    #[derive(Debug, serde::Deserialize)]
    struct MinimumGithubAsset {
        name: String,
        browser_download_url: String,
    }
    let release: MinimumGithubRelease = ureq::get(&format!(
        "https://api.github.com/repos/{owner}/{repo}/releases/latest",
        owner = owner,
        repo = repo
    ))
    .call()
    .context("GitHub API からリリース情報の取得に失敗しました")?
    .into_body()
    .read_json()?;
    let pattern =
        regex::Regex::new(pattern).context("GitHub のリリースアセットのパターンが不正です")?;
    for asset in release.assets {
        if pattern.is_match(&asset.name) {
            log::info!(
                "ダウンロード URL を見つけました: {}",
                asset.browser_download_url
            );
            return Ok(asset.browser_download_url);
        }
    }
    anyhow::bail!("ダウンロード URL を見つけられませんでした");
}
