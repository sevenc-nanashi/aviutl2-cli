use anyhow::Context;
use fs_err as fs;

use crate::{
    catalog_schema::{CatalogIndexEntry, InstallerSource},
    config::CatalogDependency,
};

pub fn load_catalog_index(
    refresh: bool,
) -> anyhow::Result<std::collections::HashMap<String, CatalogIndexEntry>> {
    std::fs::create_dir_all(".aviutl2-cli")
        .context("カタログの保存ディレクトリの作成に失敗しました")?;
    let path = std::path::PathBuf::from(".aviutl2-cli/catalog-index.json");
    if !should_reload_catalog_index(&path, refresh) {
        tracing::info!("カタログをキャッシュから読み込みます");
        let maybe_entries = fs::read_to_string(&path)
            .context("カタログの読み込みに失敗しました")
            .and_then(|content| {
                serde_json::from_str::<std::collections::HashMap<String, CatalogIndexEntry>>(
                    &content,
                )
                .context("カタログの解析に失敗しました")
            });
        match maybe_entries {
            Ok(entries) => return Ok(entries),
            Err(e) => {
                tracing::warn!("カタログの読み込みに失敗しました: {}", e);
            }
        }
    }
    let entries = fetch_catalog_index()?;
    fs::write(path, serde_json::to_string(&entries)?).context("カタログの保存に失敗しました")?;
    Ok(entries)
}

fn should_reload_catalog_index(path: &std::path::Path, refresh: bool) -> bool {
    if refresh {
        return true;
    }
    if let Ok(metadata) = fs::metadata(path)
        && let Ok(modified) = metadata.modified()
        && let Ok(elapsed) = modified.elapsed()
    {
        return elapsed.as_secs() > 3600; // 1時間以上経過している場合は再取得する
    }
    true
}

fn fetch_catalog_index() -> anyhow::Result<std::collections::HashMap<String, CatalogIndexEntry>> {
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
    let entries: Vec<CatalogIndexEntry> = response
        .into_body()
        .read_json()
        .context("カタログの解析に失敗しました")?;
    let content = entries
        .into_iter()
        .map(|entry| (entry.id.clone(), entry))
        .collect();
    Ok(content)
}

#[tracing::instrument(skip_all, fields(id = %entry.id, version = %entry.latest_version))]
pub fn install(data_root: &std::path::Path, entry: &CatalogIndexEntry) -> anyhow::Result<()> {
    run_catalog_actions(
        entry,
        data_root,
        &entry.installer.install,
        Some(&entry.installer.source),
    )?;

    Ok(())
}

pub fn sync(
    data_root: &std::path::Path,
    catalog: &std::collections::HashMap<String, CatalogIndexEntry>,
    dependencies: &[CatalogDependency],
    force_reinstall: bool,
) -> anyhow::Result<()> {
    let store_path = data_root.join("au2cli_catalog_store.json");
    let store = fs::read_to_string(&store_path)
        .ok()
        .and_then(|content| {
            serde_json::from_str::<std::collections::HashMap<String, CatalogIndexEntry>>(&content)
                .ok()
        })
        .unwrap_or_default();
    let mut new_store = std::collections::HashMap::new();
    for dependency in dependencies {
        let entry = catalog.get(&dependency.id).with_context(|| {
            format!(
                "カタログに依存関係が見つかりませんでした: {}",
                dependency.id
            )
        })?;
        new_store.insert(entry.id.clone(), entry.clone());
    }

    let keys = store
        .keys()
        .chain(new_store.keys())
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    for key in keys {
        match (store.get(&key), new_store.get(&key)) {
            (Some(old_version), Some(new_version))
                if old_version.latest_version != new_version.latest_version =>
            {
                tracing::info!(
                    "バージョンが更新されました: {} {} -> {}",
                    key,
                    old_version.latest_version,
                    new_version.latest_version
                );
                uninstall(data_root, old_version)?;
                install(data_root, new_version)?;
            }
            (Some(old_version), Some(_new_version)) if force_reinstall => {
                tracing::info!("強制再インストール: {} {}", key, old_version.latest_version);
                uninstall(data_root, old_version)?;
                install(data_root, old_version)?;
            }
            (Some(old_version), Some(_new_version)) => {
                tracing::info!("インストール済です: {} {}", key, old_version.latest_version);
            }
            (None, Some(new_version)) => {
                tracing::info!(
                    "新しいプラグインが見つかりました: {} {}",
                    key,
                    new_version.latest_version
                );
                install(data_root, new_version)?;
            }
            (Some(old_version), None) => {
                tracing::info!("プラグインが削除されました: {}", key);
                uninstall(data_root, old_version)?;
            }
            (None, None) => unreachable!(),
        }
    }

    fs::write(&store_path, serde_json::to_string(&new_store)?)
        .context("カタログストアの保存に失敗しました")?;

    Ok(())
}

fn run_catalog_actions(
    entry: &CatalogIndexEntry,
    data_root: &std::path::Path,
    actions: &[crate::catalog_schema::InstallerAction],
    install_source: Option<&InstallerSource>,
) -> anyhow::Result<()> {
    let downloaded_path: std::sync::Arc<std::sync::OnceLock<std::path::PathBuf>> =
        std::sync::Arc::new(std::sync::OnceLock::new());
    let temp_dir = tempfile::tempdir()?;
    let resolve_placeholders = |path: &str| -> String {
        path.replace("{tmp}", temp_dir.path().to_str().unwrap_or_default())
            .replace(
                "{dataDir}",
                dunce::canonicalize(data_root)
                    .unwrap_or_else(|_| data_root.to_path_buf())
                    .to_str()
                    .unwrap_or_default(),
            )
            .replace(
                "{pluginsDir}",
                dunce::canonicalize(data_root.join("Plugin"))
                    .unwrap_or_else(|_| data_root.join("Plugin"))
                    .to_str()
                    .unwrap_or_default(),
            )
            .replace(
                "{scriptsDir}",
                dunce::canonicalize(data_root.join("Script"))
                    .unwrap_or_else(|_| data_root.join("Script"))
                    .to_str()
                    .unwrap_or_default(),
            )
            .replace(
                "{download}",
                downloaded_path
                    .get()
                    .map(|p| p.to_str().unwrap_or_default())
                    .unwrap_or_default(),
            )
    };
    for (i, action) in actions.iter().enumerate() {
        let _span = tracing::info_span!("install_action", action_index = i).entered();
        match action {
            crate::catalog_schema::InstallerAction::Download {} => {
                let (filename, download_url) = match install_source
                    .ok_or_else(|| anyhow::anyhow!("インストール元が指定されていません"))?
                {
                    crate::catalog_schema::InstallerSource::Github { github } => {
                        let url = resolve_github_download_url(
                            &github.owner,
                            &github.repo,
                            &github.pattern,
                        )?;
                        let filename = url
                            .split('/')
                            .next_back()
                            .ok_or_else(|| anyhow::anyhow!("ダウンロード URL が不正です"))?
                            .to_string();
                        (Some(filename), url)
                    }
                    crate::catalog_schema::InstallerSource::GoogleDrive { google_drive } => (
                        None,
                        format!(
                            "https://drive.google.com/uc?export=download&id={id}",
                            id = google_drive.id
                        ),
                    ),
                    crate::catalog_schema::InstallerSource::Direct { direct } => {
                        let filename = direct.split('/').next_back().map(|s| s.to_string());
                        (filename, direct.clone())
                    }
                    crate::catalog_schema::InstallerSource::Booth { booth: _ } => {
                        anyhow::bail!("Booth からのインストールはサポートされていません");
                    }
                };
                let response = ureq::get(download_url)
                    .call()
                    .context("ファイルのダウンロードに失敗しました")?;
                if response.status() != 200 {
                    anyhow::bail!(
                        "ファイルのダウンロードに失敗しました: HTTP {}",
                        response.status()
                    );
                }
                let content_disposition = response
                    .headers()
                    .get("content-disposition")
                    .map(|cd| String::from_utf8_lossy(cd.as_bytes()).to_string());
                let filename = filename
                    .or_else(|| {
                        content_disposition.as_ref().and_then(|cd| {
                            static PATTERN: lazy_regex::Lazy<regex::Regex> =
                                lazy_regex::lazy_regex!(r#"filename="([^"]+)""#);
                            PATTERN
                                .captures(cd)
                                .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
                        })
                    })
                    .unwrap_or_else(|| format!("{}.download", entry.id));

                let mut reader = response.into_body().into_reader();
                let download_path = temp_dir.path().join(&filename);
                let mut file = fs::File::create(&download_path)
                    .context("ダウンロードしたファイルの保存に失敗しました")?;
                std::io::copy(&mut reader, &mut file)
                    .context("ダウンロードしたファイルの保存に失敗しました")?;
                tracing::info!("ファイルを保存しました: {}", download_path.display());
                downloaded_path
                    .set(download_path)
                    .map_err(|_| anyhow::anyhow!("複数のファイルはダウンロードできません"))?;
            }
            crate::catalog_schema::InstallerAction::Extract {} => {
                let file = fs::File::open(downloaded_path.get().ok_or_else(|| {
                    anyhow::anyhow!("ダウンロードが完了していない状態で展開しようとしました")
                })?)
                .context("ダウンロードしたファイルの読み込みに失敗しました")?;
                let mut archive = zip::ZipArchive::new(file)
                    .context("ダウンロードしたファイルの展開に失敗しました")?;
                for i in 0..archive.len() {
                    let mut file = archive.by_index(i)?;
                    let out_path = temp_dir.path().join(file.mangled_name());
                    if file.is_dir() {
                        fs::create_dir_all(&out_path)?;
                    } else {
                        if let Some(parent) = out_path.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        let mut out_file = fs::File::create(&out_path)?;
                        std::io::copy(&mut file, &mut out_file)?;
                    }
                }
                tracing::info!("ファイルを展開しました: {}", temp_dir.path().display());
            }
            crate::catalog_schema::InstallerAction::ExtractSfx {} => {
                let mut file = fs::File::open(downloaded_path.get().ok_or_else(|| {
                    anyhow::anyhow!("ダウンロードが完了していない状態で展開しようとしました")
                })?)
                .context("ダウンロードしたファイルの読み込みに失敗しました")?;
                let finder = aho_corasick::AhoCorasick::new([b"\x37\x7A\xBC\xAF\x27\x1C"])?;
                let found = finder
                    .stream_find_iter(&mut file)
                    .next()
                    .context("展開に失敗しました")??;
                let start_position = found.start();
                let mut file = OffsetReader::new(file, start_position as u64);
                sevenz_rust2::decompress_with_extract_fn(
                    &mut file,
                    temp_dir.path(),
                    |entry, reader, dest| {
                        static EXCLUDES: &[&str] = &[
                            "VC_redist.x64.exe",
                            "VC_redist.x86.exe",
                            "auo_setup.ini",
                            "auo_setup2.exe",
                        ];
                        if EXCLUDES
                            .iter()
                            .any(|exclude| dest.file_name().is_some_and(|name| name == *exclude))
                        {
                            tracing::info!("展開をスキップ: {}", dest.display());
                            // FIXME:
                            // 読み出しまでスキップすると壊れるので、読み出しは行うが書き出しをスキップする
                            // return Ok(false);
                            let mut buffer = [0u8; 1024];
                            while reader.read(&mut buffer)? > 0 {}
                            return Ok(true);
                        }
                        if let Some(parent) = dest.parent()
                            && !parent.exists()
                        {
                            tracing::info!("ディレクトリを作成: {}", parent.display());
                            fs::create_dir_all(parent)?;
                        }
                        tracing::info!("展開中: {}", dest.display());
                        sevenz_rust2::default_entry_extract_fn(entry, reader, dest)
                    },
                )?;
            }
            crate::catalog_schema::InstallerAction::Copy { from, to } => {
                let from_path: std::path::PathBuf = resolve_placeholders(from).into();
                let to_dir_path: std::path::PathBuf = resolve_placeholders(to).into();
                let to_path = to_dir_path.join(
                    from_path
                        .file_name()
                        .context("コピー元のファイル名が不正です")?,
                );
                if let Some(parent) = to_path.parent()
                    && !parent.exists()
                {
                    tracing::info!("ディレクトリを作成: {}", parent.display());
                    fs::create_dir_all(parent)?;
                }
                tracing::info!(
                    "ファイルをコピー: {} -> {}",
                    from_path.display(),
                    to_path.display()
                );
                fs::copy(&from_path, &to_path).with_context(|| {
                    format!(
                        "ファイルのコピーに失敗しました: {} -> {}",
                        from_path.display(),
                        to_path.display()
                    )
                })?;
            }
            crate::catalog_schema::InstallerAction::Delete { path } => {
                let path: std::path::PathBuf = resolve_placeholders(path).into();
                if path.exists() {
                    tracing::info!("ファイルを削除: {}", path.display());
                    if path.is_dir() {
                        fs::remove_dir_all(&path).with_context(|| {
                            format!("ディレクトリの削除に失敗しました: {}", path.display())
                        })?;
                    } else {
                        fs::remove_file(&path).with_context(|| {
                            format!("ファイルの削除に失敗しました: {}", path.display())
                        })?;
                    }
                } else {
                    tracing::info!("削除するファイルが見つかりませんでした: {}", path.display());
                }
            }
            crate::catalog_schema::InstallerAction::Run {
                path,
                args,
                elevate,
            } => {
                if elevate.unwrap_or(false) {
                    anyhow::bail!("管理者権限での実行はサポートされていません");
                }
                let path: std::path::PathBuf = resolve_placeholders(path).into();
                let args = args
                    .iter()
                    .map(|arg| resolve_placeholders(arg))
                    .collect::<Vec<_>>();
                tracing::info!("コマンドを実行: {} {}", path.display(), args.join(" "));
                std::process::Command::new(path)
                    .args(args)
                    .status()
                    .with_context(|| "コマンドの実行に失敗しました")?;
            }
            crate::catalog_schema::InstallerAction::RunAuoSetup { path: _ } => {
                // NOTE:
                // auo_setup.exeをわざわざ走らせるのは面倒なので、Plugin下のコピーという単純な処理で代替する

                let from_plugin_dir = temp_dir.path().join("Plugin");
                let to_plugin_dir = data_root.join("Plugin");
                anyhow::ensure!(
                    from_plugin_dir.exists(),
                    "Plugin ディレクトリが見つかりませんでした: {}",
                    from_plugin_dir.display()
                );
                fs_extra::dir::copy(
                    &from_plugin_dir,
                    &to_plugin_dir,
                    &fs_extra::dir::CopyOptions::new()
                        .content_only(true)
                        .overwrite(true),
                )
                .with_context(|| {
                    format!(
                        "Plugin ディレクトリへのコピーに失敗しました: {} -> {}",
                        from_plugin_dir.display(),
                        to_plugin_dir.display()
                    )
                })?;
            }
        }
    }
    Ok(())
}

struct OffsetReader<R: std::io::Read + std::io::Seek> {
    inner: R,
    offset: u64,
}
impl<R: std::io::Read + std::io::Seek> OffsetReader<R> {
    fn new(inner: R, offset: u64) -> Self {
        Self { inner, offset }
    }
}
impl<R: std::io::Read + std::io::Seek> std::io::Read for OffsetReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}
impl<R: std::io::Read + std::io::Seek> std::io::Seek for OffsetReader<R> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            std::io::SeekFrom::Start(offset) => self.offset.checked_add(offset),
            std::io::SeekFrom::Current(offset) => {
                let current_pos = self.inner.stream_position()?;
                if offset >= 0 {
                    current_pos.checked_add(offset as u64)
                } else {
                    current_pos.checked_sub((-offset) as u64)
                }
            }
            std::io::SeekFrom::End(offset) => {
                let end_pos = self.inner.seek(std::io::SeekFrom::End(0))?;
                if offset >= 0 {
                    end_pos.checked_add(offset as u64)
                } else {
                    end_pos.checked_sub((-offset) as u64)
                }
            }
        }
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "seek position overflow")
        })?;
        self.inner.seek(std::io::SeekFrom::Start(new_pos))
    }
}

fn resolve_github_download_url(owner: &str, repo: &str, pattern: &str) -> anyhow::Result<String> {
    #[derive(Debug, Clone, serde::Deserialize)]
    struct MinimumGithubRelease {
        assets: Vec<MinimumGithubAsset>,
    }
    #[derive(Debug, Clone, serde::Deserialize)]
    struct MinimumGithubAsset {
        name: String,
        browser_download_url: String,
    }
    let release = match ureq::get(&format!(
        "https://api.github.com/repos/{owner}/{repo}/releases/latest",
        owner = owner,
        repo = repo
    ))
    .call()
    {
        Ok(response) => response
            .into_body()
            .read_json::<MinimumGithubRelease>()
            .context("GitHub のリリース情報の解析に失敗しました")?,
        Err(ureq::Error::StatusCode(404)) => {
            let releases = ureq::get(&format!(
                "https://api.github.com/repos/{owner}/{repo}/releases",
                owner = owner,
                repo = repo
            ))
            .call()
            .context("GitHub のリリース情報の取得に失敗しました")?
            .into_body()
            .read_json::<Vec<MinimumGithubRelease>>()
            .context("GitHub のリリース情報の解析に失敗しました")?;
            if let Some(release) = releases.first() {
                release.clone()
            } else {
                anyhow::bail!("GitHub のリリース情報が見つかりませんでした: {owner}/{repo}");
            }
        }
        Err(e) => {
            return Err(e).context("GitHub のリリース情報の取得に失敗しました");
        }
    };
    let pattern =
        regex::Regex::new(pattern).context("GitHub のリリースアセットのパターンが不正です")?;
    for asset in release.assets {
        if pattern.is_match(&asset.name) {
            tracing::info!(
                "ダウンロード URL を見つけました: {}",
                asset.browser_download_url
            );
            return Ok(asset.browser_download_url);
        }
    }
    anyhow::bail!("ダウンロード URL を見つけられませんでした");
}

#[tracing::instrument(skip_all, fields(id = %entry.id, version = %entry.latest_version))]
pub fn uninstall(data_root: &std::path::Path, entry: &CatalogIndexEntry) -> anyhow::Result<()> {
    run_catalog_actions(entry, data_root, &entry.installer.uninstall, None)?;
    Ok(())
}
