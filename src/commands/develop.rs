use anyhow::{Context, Result, bail};
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;

use crate::config::load_config;
use crate::config::{BuildCommand, Config, PlacementMethod};
use crate::util::{copy_to_destination, development_dir, find_aviutl2_data_dir, resolve_source};

pub struct ResolvedArtifact {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub build_plan: ResolvedBuild,
    pub placement_method: PlacementMethod,
}

pub struct ResolvedBuild {
    pub commands: Vec<String>,
    pub group: Option<String>,
}

pub fn run(
    profile: Option<String>,
    skip_start: bool,
    refresh: bool,
    args: Vec<String>,
) -> Result<()> {
    let config = load_config()?;
    let dev = config
        .development
        .as_ref()
        .context("development 設定が必要です")?;
    warn_if_prepare_snapshot_changed(&config, &dev.aviutl2_version)?;
    let install_dir = development_dir(dev)?;
    let profile = profile
        .as_deref()
        .or(dev.profile.as_deref())
        .unwrap_or("debug");
    run_optional_commands(dev.prebuild.as_ref(), config.build_group.as_ref())?;
    let artifacts = resolve_artifacts(&config, Some(profile), None, refresh)?;
    let data_dir = find_aviutl2_data_dir(&install_dir)?;
    let mut anything_copied = false;
    let mut executed_groups = HashSet::new();
    for artifact in artifacts {
        run_build_plan(&artifact.build_plan, &mut executed_groups)?;
        let dest = data_dir.join(&artifact.destination);
        let needs_copy = matches!(artifact.placement_method, PlacementMethod::Copy);
        if needs_copy {
            copy_to_destination(&artifact.source, &dest, true)?;
            anything_copied = true;
        }
    }
    if anything_copied {
        log::info!("成果物を配置しました");
    }
    run_optional_commands(dev.postbuild.as_ref(), config.build_group.as_ref())?;

    if !skip_start {
        let aviutl_exe = data_dir.parent().unwrap_or(&data_dir).join("aviutl2.exe");
        if aviutl_exe.exists() {
            log::info!("AviUtl2 を起動します: {}", aviutl_exe.display());
            Command::new(aviutl_exe)
                .args(args)
                .spawn()
                .with_context(|| "AviUtl2 の起動に失敗しました")?;
        } else {
            log::warn!("AviUtl2.exe が見つかりません: {}", aviutl_exe.display());
        }
    }
    Ok(())
}

fn warn_if_prepare_snapshot_changed(config: &Config, aviutl2_version: &str) -> Result<()> {
    let Some(snapshot) = super::prepare::load_prepare_snapshot()? else {
        return Ok(());
    };
    let mut ordered = std::collections::BTreeMap::new();
    for (name, artifact) in &config.artifacts {
        ordered.insert(name.clone(), artifact.clone());
    }
    let current = super::prepare::PrepareSnapshot {
        aviutl2_version: aviutl2_version.to_string(),
        artifacts: ordered,
    };
    if snapshot.aviutl2_version != current.aviutl2_version
        || snapshot.artifacts != current.artifacts
    {
        log::warn!(
            "prepare 実行時の設定と現在の設定が異なります。必要なら `au2 prepare` を再実行してください。"
        );
    }
    Ok(())
}

pub fn resolve_artifacts(
    config: &Config,
    profile: Option<&str>,
    include: Option<&[String]>,
    refresh: bool,
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
        let enabled = profile_data
            .and_then(|p| p.enabled)
            .or(artifact.enabled)
            .unwrap_or(true);
        if !enabled {
            continue;
        }
        let source = profile_data
            .and_then(|p| p.source.clone())
            .or_else(|| artifact.source.clone())
            .with_context(|| format!("artifacts.{}.source が必要です", name))?;
        let source = resolve_source(&source, refresh)?;
        let build = profile_data
            .and_then(|p| p.build.clone())
            .or_else(|| artifact.build.clone());
        let build_plan = resolve_build_plan(build.as_ref(), config.build_group.as_ref())?;
        let placement_method = artifact
            .placement_method
            .unwrap_or(PlacementMethod::Symlink);
        resolved.push(ResolvedArtifact {
            source,
            destination: PathBuf::from(&artifact.destination),
            build_plan,
            placement_method,
        });
    }
    Ok(resolved)
}

pub fn run_build_plan(plan: &ResolvedBuild, executed_groups: &mut HashSet<String>) -> Result<()> {
    if let Some(group) = &plan.group {
        if executed_groups.contains(group) {
            return Ok(());
        }
        run_build_commands(&plan.commands)?;
        executed_groups.insert(group.clone());
        return Ok(());
    }
    run_build_commands(&plan.commands)
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

pub(crate) fn run_optional_commands(
    commands: Option<&BuildCommand>,
    build_groups: Option<&std::collections::HashMap<String, BuildCommand>>,
) -> Result<()> {
    let commands = resolve_build_commands(commands, build_groups)?;
    if !commands.is_empty() {
        run_build_commands(&commands)?;
    }
    Ok(())
}

fn resolve_build_commands(
    command: Option<&BuildCommand>,
    build_groups: Option<&std::collections::HashMap<String, BuildCommand>>,
) -> Result<Vec<String>> {
    let mut visiting = std::collections::HashSet::new();
    resolve_build_commands_inner(command, build_groups, &mut visiting)
}

fn resolve_build_plan(
    command: Option<&BuildCommand>,
    build_groups: Option<&std::collections::HashMap<String, BuildCommand>>,
) -> Result<ResolvedBuild> {
    let commands = resolve_build_commands(command, build_groups)?;
    let group = match command {
        Some(BuildCommand::Group(group_ref)) => Some(group_ref.group.clone()),
        _ => None,
    };
    Ok(ResolvedBuild { commands, group })
}

fn resolve_build_commands_inner(
    command: Option<&BuildCommand>,
    build_groups: Option<&std::collections::HashMap<String, BuildCommand>>,
    visiting: &mut std::collections::HashSet<String>,
) -> Result<Vec<String>> {
    match command {
        None => Ok(Vec::new()),
        Some(BuildCommand::Single(cmd)) => Ok(vec![cmd.clone()]),
        Some(BuildCommand::Multiple(cmds)) => Ok(cmds.clone()),
        Some(BuildCommand::Group(group_ref)) => {
            let build_groups = build_groups.context("build_group が定義されていません")?;
            let name = &group_ref.group;
            let group = build_groups
                .get(name)
                .with_context(|| format!("build_group.{} が見つかりません", name))?;
            if !visiting.insert(name.clone()) {
                bail!("build_group の循環参照を検出しました: {}", name);
            }
            let resolved = resolve_build_commands_inner(Some(group), Some(build_groups), visiting);
            visiting.remove(name);
            resolved
        }
    }
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
