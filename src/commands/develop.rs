use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use std::process::Command;

use crate::config::load_config;
use crate::config::{Config, PlacementMethod};
use crate::util::{create_symlink, find_aviutl2_data_dir};

pub struct ResolvedArtifact {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub build_commands: Vec<String>,
    pub placement_method: PlacementMethod,
}

pub fn run(profile: Option<String>, skip_start: bool) -> Result<()> {
    let config = load_config()?;
    let dev = config
        .development
        .as_ref()
        .context("development 設定が必要です")?;
    let install_dir = PathBuf::from(dev.install_dir.as_deref().unwrap_or("./development"));
    let profile = profile
        .as_deref()
        .or(dev.profile.as_deref())
        .unwrap_or("debug");
    let artifacts = resolve_artifacts(&config, Some(profile), None)?;
    let data_dir = find_aviutl2_data_dir(&install_dir)?;
    for artifact in artifacts {
        run_build_commands(&artifact.build_commands)?;
        if !artifact.source.exists() {
            log::warn!("source が見つかりません: {}", artifact.source.display());
            continue;
        }
        let dest = data_dir.join(&artifact.destination);
        match artifact.placement_method {
            PlacementMethod::Symlink => create_symlink(&artifact.source, &dest, false)?,
            PlacementMethod::Copy => {
                log::warn!(
                    "develop では copy を使わないため symlink を作成します: {}",
                    dest.display()
                );
                create_symlink(&artifact.source, &dest, false)?;
            }
        }
    }
    log::info!("成果物を配置しました");

    if !skip_start {
        let aviutl_exe = data_dir.parent().unwrap_or(&data_dir).join("aviutl2.exe");
        if aviutl_exe.exists() {
            log::info!("AviUtl2 を起動します: {}", aviutl_exe.display());
            Command::new(aviutl_exe)
                .spawn()
                .with_context(|| "AviUtl2 の起動に失敗しました")?;
        } else {
            log::warn!("AviUtl2.exe が見つかりません: {}", aviutl_exe.display());
        }
    }
    Ok(())
}

pub fn resolve_artifacts(
    config: &Config,
    profile: Option<&str>,
    include: Option<&[String]>,
) -> Result<Vec<ResolvedArtifact>> {
    let mut resolved = Vec::new();
    for (name, artifact) in &config.artifacts {
        if let Some(include) = include
            && !include.iter().any(|item| item == name)
        {
            continue;
        }
        let profile_data = profile.and_then(|p| {
            artifact
                .profiles
                .as_ref()
                .and_then(|profiles| profiles.get(p))
        });
        let source = profile_data
            .and_then(|p| p.source.clone())
            .or_else(|| artifact.source.clone())
            .with_context(|| format!("artifacts.{}.source が必要です", name))?;
        let build = profile_data
            .and_then(|p| p.build.clone())
            .or_else(|| artifact.build.clone());
        let build_commands = build.map(|cmd| cmd.as_vec()).unwrap_or_default();
        let placement_method = artifact
            .placement_method
            .unwrap_or(PlacementMethod::Symlink);
        resolved.push(ResolvedArtifact {
            source: PathBuf::from(source),
            destination: PathBuf::from(&artifact.destination),
            build_commands,
            placement_method,
        });
    }
    Ok(resolved)
}

pub fn run_build_commands(commands: &[String]) -> Result<()> {
    for cmd in commands {
        log::info!("コマンド実行: {}", cmd);
        let status = run_shell_command(cmd)?;
        if !status.success() {
            bail!("ビルドコマンドが失敗しました: {}", cmd);
        }
    }
    Ok(())
}

fn run_shell_command(command: &str) -> Result<std::process::ExitStatus> {
    if cfg!(windows) {
        Command::new("cmd")
            .args(["/C", command])
            .status()
            .map_err(Into::into)
    } else {
        Command::new("sh")
            .args(["-c", command])
            .status()
            .map_err(Into::into)
    }
}
