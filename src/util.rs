use anyhow::{Context, Result, bail};
use fs_err as fs;
use fs_err::File;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;
use zip::write::FileOptions;

pub fn safe_join(base: &Path, entry_name: &str) -> Result<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in Path::new(entry_name).components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            _ => bail!("zip 内の不正なパスを検出しました: {}", entry_name),
        }
    }
    Ok(base.join(normalized))
}

pub fn extract_zip(zip_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = File::open(zip_path)
        .with_context(|| format!("zip の読み込みに失敗しました: {}", zip_path.display()))?;
    let mut archive = zip::ZipArchive::new(file).context("zip の解析に失敗しました")?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let entry_name = entry.name();
        let out_path = safe_join(dest_dir, entry_name)?;
        if entry.is_dir() {
            if out_path.exists() && !out_path.is_dir() {
                remove_path(&out_path)?;
            }
            fs::create_dir_all(&out_path)?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if out_path.exists() {
            remove_path(&out_path)?;
        }
        let mut out_file = File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out_file)?;
    }
    Ok(())
}

pub fn create_zip(source_dir: &Path, zip_path: &Path) -> Result<()> {
    let file = File::create(zip_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::<()>::default().compression_method(zip::CompressionMethod::Deflated);
    let base = source_dir;
    for entry in WalkDir::new(source_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let rel = path.strip_prefix(base)?;
        let name = rel
            .components()
            .filter_map(|c| match c {
                Component::Normal(part) => Some(part),
                _ => None,
            })
            .collect::<PathBuf>();
        let name = path_to_slash(&name);
        zip.start_file(name, options)?;
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        zip.write_all(&buffer)?;
    }
    zip.finish()?;
    Ok(())
}

pub fn remove_path(path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err.into()),
    };
    if metadata.file_type().is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn create_symlink(source: &Path, destination: &Path, force: bool) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let metadata = match fs::symlink_metadata(destination) {
        Ok(metadata) => Some(metadata),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => return Err(err.into()),
    };
    if let Some(metadata) = metadata {
        if metadata.file_type().is_symlink() || force {
            remove_path(destination)?;
        } else {
            bail!(
                "既存ファイルがあるため作成できません（--force で上書き）: {}",
                destination.display()
            );
        }
    }
    if let Err(err) = create_symlink_inner(source, destination) {
        if err.kind() == std::io::ErrorKind::AlreadyExists {
            let metadata = match fs::symlink_metadata(destination) {
                Ok(metadata) => Some(metadata),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
                Err(err) => return Err(err.into()),
            };
            if let Some(metadata) = metadata {
                if metadata.file_type().is_symlink() || force {
                    remove_path(destination)?;
                } else {
                    bail!(
                        "既存ファイルがあるため作成できません（--force で上書き）: {}",
                        destination.display()
                    );
                }
            }
            create_symlink_inner(source, destination)?;
        } else {
            return Err(err.into());
        }
    }
    log::info!(
        "symlink を作成しました: {} -> {}",
        destination.display(),
        source.display()
    );
    Ok(())
}

fn create_symlink_inner(source: &Path, destination: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(source, destination)
    }
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(source, destination)
    }
}

pub fn copy_to_destination(source: &Path, destination: &Path, force: bool) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let metadata = match fs::symlink_metadata(destination) {
        Ok(metadata) => Some(metadata),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => return Err(err.into()),
    };
    if let Some(metadata) = metadata {
        if metadata.file_type().is_symlink() || force {
            remove_path(destination)?;
        } else {
            bail!(
                "既存ファイルがあるため作成できません（--force で上書き）: {}",
                destination.display()
            );
        }
    }
    fs::copy(source, destination)?;
    log::info!(
        "コピーしました: {} -> {}",
        source.display(),
        destination.display()
    );
    Ok(())
}

pub fn find_aviutl2_data_dir(install_dir: &Path) -> Result<PathBuf> {
    if !install_dir.exists() {
        bail!(
            "AviUtl2 のインストール先が見つかりません: {}",
            install_dir.display()
        );
    }
    for entry in WalkDir::new(install_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if name.eq_ignore_ascii_case("aviutl2.exe") {
            let parent = entry
                .path()
                .parent()
                .context("aviutl2.exe の親ディレクトリが見つかりません")?;
            return Ok(parent.join("data"));
        }
    }
    bail!("aviutl2.exe が見つかりません: {}", install_dir.display());
}

fn path_to_slash(path: &Path) -> String {
    let mut parts = Vec::new();
    for component in path.components() {
        if let Component::Normal(part) = component {
            parts.push(part.to_string_lossy());
        }
    }
    parts.join("/")
}

pub fn fill_template(template: &str, project: &crate::config::Project) -> String {
    template
        .replace("{name}", &project.name)
        .replace("{version}", &project.version)
}

pub fn development_dir(dev: &crate::config::Development) -> Result<PathBuf> {
    if let Some(install_dir) = dev.install_dir.as_deref() {
        return Ok(PathBuf::from(install_dir));
    }
    let mut base = cli_dir()?;
    base.push("development");
    Ok(base)
}

pub fn resolve_source(source: &str, refresh: bool) -> Result<PathBuf> {
    if is_http_url(source) {
        let downloaded = download_http_source(source, refresh)?;
        return Ok(downloaded);
    }
    Ok(PathBuf::from(source))
}

fn is_http_url(source: &str) -> bool {
    source.starts_with("http://") || source.starts_with("https://")
}

fn download_http_source(url: &str, refresh: bool) -> Result<PathBuf> {
    let file_name = filename_from_url(url);
    let cache_dir = http_cache_dir()?;
    fs::create_dir_all(&cache_dir)?;
    let hash = hash_url(url);
    let cache_path = cache_dir.join(format!("{hash}_{file_name}"));
    if cache_path.exists() && !refresh {
        log::info!("source のキャッシュを使用します: {} -> {}", url, cache_path.display());
        return Ok(cache_path);
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut temp_path = cache_dir.join(format!("{ts}_{file_name}.partial"));

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .max_redirects(5)
        .build()
        .into();
    let response = agent
        .get(url)
        .header("User-Agent", "aviutl2-cli")
        .call()
        .with_context(|| format!("source のダウンロードに失敗しました: {url}"))?;
    let status = response.status();
    if !status.is_success() {
        bail!("source のダウンロードに失敗しました: {} ({})", url, status);
    }

    let (_parts, body) = response.into_parts();
    let mut reader = body.into_reader();
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;

    if temp_path.exists() {
        temp_path = cache_dir.join(format!("{ts}_{}_1.partial", file_name));
    }
    let mut file = File::create(&temp_path)?;
    file.write_all(&buf)?;
    if cache_path.exists() {
        remove_path(&cache_path)?;
    }
    fs::rename(&temp_path, &cache_path)?;
    log::info!(
        "source をダウンロードしました: {} -> {}",
        url,
        cache_path.display()
    );
    Ok(cache_path)
}

fn filename_from_url(url: &str) -> String {
    let url = url.split('#').next().unwrap_or(url);
    let url = url.split('?').next().unwrap_or(url);
    let name = url.rsplit('/').next().unwrap_or("download");
    if name.is_empty() {
        "download".to_string()
    } else {
        name.to_string()
    }
}

pub fn release_stage_dir() -> Result<PathBuf> {
    let mut base = cli_dir()?;
    base.push("release-stage");
    Ok(base)
}

fn http_cache_dir() -> Result<PathBuf> {
    let mut base = cli_dir()?;
    base.push("cache");
    Ok(base)
}

fn cli_dir() -> Result<PathBuf> {
    let mut base = std::env::current_dir().context("カレントディレクトリの取得に失敗しました")?;
    base.push(".aviutl2-cli");
    Ok(base)
}

fn hash_url(url: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
