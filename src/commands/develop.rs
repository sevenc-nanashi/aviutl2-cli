use anyhow::{Context, Result, bail, ensure};
use encoding_rs::{CoderResult, Decoder, SHIFT_JIS};
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime};

use crate::config::{BuildCommand, Config, ConfigLoadOpts, PlacementMethod, load_config};
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
    detach: bool,
    refresh: bool,
    args: Vec<String>,
    opts: &ConfigLoadOpts,
) -> Result<()> {
    let config = load_config(opts)?;
    let dev = config
        .development
        .as_ref()
        .context("development 設定が必要です")?;
    warn_if_prepare_snapshot_changed(&config, &dev.aviutl2_version)?;
    let install_dir = development_dir(dev)?;
    let profile = profile.as_deref().unwrap_or(&dev.profile);
    run_optional_commands(Some(&dev.prebuild), &config.build_group)?;
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
        tracing::info!("成果物を配置しました");
    }
    run_optional_commands(Some(&dev.postbuild), &config.build_group)?;

    if !skip_start {
        let aviutl_exe = data_dir.parent().unwrap_or(&data_dir).join("aviutl2.exe");
        if aviutl_exe.exists() {
            tracing::info!("AviUtl2 を起動します: {}", aviutl_exe.display());
            Command::new(aviutl_exe)
                .args(args)
                .spawn()
                .with_context(|| "AviUtl2 の起動に失敗しました")?;
            if !detach {
                follow_latest_log(&data_dir.join("log"), &mut std::io::stdout().lock())?;
            }
        } else {
            tracing::warn!("AviUtl2.exe が見つかりません: {}", aviutl_exe.display());
        }
    }
    Ok(())
}

struct LogTailer {
    path: Option<PathBuf>,
    file: Option<File>,
    position: u64,
    decoder: Decoder,
}

impl LogTailer {
    fn new() -> Self {
        Self {
            path: None,
            file: None,
            position: 0,
            decoder: SHIFT_JIS.new_decoder_without_bom_handling(),
        }
    }

    fn poll(&mut self, log_dir: &Path, output: &mut impl Write) -> Result<()> {
        let latest = latest_log_file(log_dir)?;
        if latest != self.path {
            self.path = latest.clone();
            self.file = None;
            self.position = 0;
            self.decoder = SHIFT_JIS.new_decoder_without_bom_handling();
            if let Some(path) = latest {
                tracing::info!("ログを監視します: {}", path.display());
                let mut file = File::open(&path).with_context(|| {
                    format!("ログファイルを開けませんでした: {}", path.display())
                })?;
                self.position = file.seek(SeekFrom::End(0))?;
                self.file = Some(file);
            }
            return Ok(());
        }

        let Some(file) = self.file.as_mut() else {
            return Ok(());
        };
        let length = file.metadata()?.len();
        if length < self.position {
            self.position = file.seek(SeekFrom::Start(0))?;
            self.decoder = SHIFT_JIS.new_decoder_without_bom_handling();
        }
        file.seek(SeekFrom::Start(self.position))?;
        let mut input = Vec::new();
        file.read_to_end(&mut input)?;
        if !input.is_empty() {
            let mut decoded = String::with_capacity(input.len() * 3 + 8);
            let (result, read, _) = self.decoder.decode_to_string(&input, &mut decoded, false);
            ensure!(
                result == CoderResult::InputEmpty,
                "Shift_JISログの変換バッファが不足しました"
            );
            ensure!(
                read == input.len(),
                "Shift_JISログを最後まで変換できませんでした"
            );
            output.write_all(decoded.as_bytes())?;
            output.flush()?;
            self.position += input.len() as u64;
        }
        Ok(())
    }
}

fn latest_log_file(log_dir: &Path) -> Result<Option<PathBuf>> {
    if !log_dir.exists() {
        return Ok(None);
    }
    let mut latest: Option<(SystemTime, PathBuf)> = None;
    for entry in fs_err::read_dir(log_dir).with_context(|| {
        format!(
            "ログディレクトリを読み込めませんでした: {}",
            log_dir.display()
        )
    })? {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_file()
            || !path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("log"))
        {
            continue;
        }
        let modified = entry.metadata()?.modified()?;
        if latest
            .as_ref()
            .is_none_or(|current| (modified, &path) > (current.0, &current.1))
        {
            latest = Some((modified, path));
        }
    }
    Ok(latest.map(|(_, path)| path))
}

fn follow_latest_log(log_dir: &Path, output: &mut impl Write) -> Result<()> {
    let mut tailer = LogTailer::new();
    loop {
        tailer.poll(log_dir, output)?;
        thread::sleep(Duration::from_millis(100));
    }
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
        tracing::warn!(
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
        let build_plan = resolve_build_plan(build.as_ref(), &config.build_group)?;
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
        tracing::info!("コマンド実行: {}", cmd);
        let status = run_shell_command(cmd)?;
        if !status.success() {
            bail!("ビルドコマンドが失敗しました: {}", cmd);
        }
    }
    Ok(())
}

pub(crate) fn run_optional_commands(
    commands: Option<&BuildCommand>,
    build_groups: &std::collections::HashMap<String, BuildCommand>,
) -> Result<()> {
    let commands = resolve_build_commands(commands, build_groups)?;
    if !commands.is_empty() {
        run_build_commands(&commands)?;
    }
    Ok(())
}

fn resolve_build_commands(
    command: Option<&BuildCommand>,
    build_groups: &std::collections::HashMap<String, BuildCommand>,
) -> Result<Vec<String>> {
    let mut visiting = std::collections::HashSet::new();
    resolve_build_commands_inner(command, build_groups, &mut visiting)
}

fn resolve_build_plan(
    command: Option<&BuildCommand>,
    build_groups: &std::collections::HashMap<String, BuildCommand>,
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
    build_groups: &std::collections::HashMap<String, BuildCommand>,
    visiting: &mut std::collections::HashSet<String>,
) -> Result<Vec<String>> {
    match command {
        None => Ok(Vec::new()),
        Some(BuildCommand::Single(cmd)) => Ok(vec![cmd.clone()]),
        Some(BuildCommand::Multiple(cmds)) => Ok(cmds.clone()),
        Some(BuildCommand::Group(group_ref)) => {
            let name = &group_ref.group;
            let group = build_groups
                .get(name)
                .with_context(|| format!("build_group.{} が見つかりません", name))?;
            if !visiting.insert(name.clone()) {
                bail!("build_group の循環参照を検出しました: {}", name);
            }
            let resolved = resolve_build_commands_inner(Some(group), build_groups, visiting);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::OpenOptions;

    #[test]
    fn log_tailer_skips_existing_content_and_copies_appended_content() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let log_dir = temp.path().join("log");
        fs_err::create_dir_all(&log_dir)?;
        let log_path = log_dir.join("aviutl2.log");
        fs_err::write(&log_path, b"existing\n")?;

        let mut tailer = LogTailer::new();
        let mut output = Vec::new();
        tailer.poll(&log_dir, &mut output)?;
        assert!(output.is_empty());

        OpenOptions::new()
            .append(true)
            .open(&log_path)?
            .write_all(b"appended\n")?;
        tailer.poll(&log_dir, &mut output)?;
        assert_eq!(output, "appended\n".as_bytes());
        Ok(())
    }

    #[test]
    fn log_tailer_decodes_shift_jis_across_append_boundaries() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let log_dir = temp.path().join("log");
        fs_err::create_dir_all(&log_dir)?;
        let log_path = log_dir.join("aviutl2.log");
        fs_err::write(&log_path, b"")?;

        let mut tailer = LogTailer::new();
        let mut output = Vec::new();
        tailer.poll(&log_dir, &mut output)?;

        let (encoded, _, had_errors) = SHIFT_JIS.encode("日本語\n");
        assert!(!had_errors);
        let split = 1;
        OpenOptions::new()
            .append(true)
            .open(&log_path)?
            .write_all(&encoded[..split])?;
        tailer.poll(&log_dir, &mut output)?;
        assert!(output.is_empty());

        OpenOptions::new()
            .append(true)
            .open(&log_path)?
            .write_all(&encoded[split..])?;
        tailer.poll(&log_dir, &mut output)?;
        assert_eq!(output, "日本語\n".as_bytes());
        Ok(())
    }

    #[test]
    fn log_tailer_switches_to_a_newer_log_from_its_end() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let log_dir = temp.path().join("log");
        fs_err::create_dir_all(&log_dir)?;
        let first_path = log_dir.join("first.log");
        fs_err::write(&first_path, b"first existing\n")?;

        let mut tailer = LogTailer::new();
        let mut output = Vec::new();
        tailer.poll(&log_dir, &mut output)?;

        thread::sleep(Duration::from_millis(20));
        let second_path = log_dir.join("second.log");
        fs_err::write(&second_path, b"second existing\n")?;
        tailer.poll(&log_dir, &mut output)?;
        assert!(output.is_empty());

        OpenOptions::new()
            .append(true)
            .open(&second_path)?
            .write_all(b"second appended\n")?;
        tailer.poll(&log_dir, &mut output)?;
        assert_eq!(output, "second appended\n".as_bytes());
        Ok(())
    }

    #[test]
    fn latest_log_file_ignores_non_log_files() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let log_dir = temp.path().join("log");
        assert_eq!(latest_log_file(&log_dir)?, None);

        fs_err::create_dir_all(&log_dir)?;
        fs_err::write(log_dir.join("newest.txt"), b"ignored")?;
        fs_err::write(log_dir.join("aviutl2.LOG"), b"log")?;
        assert_eq!(
            latest_log_file(&log_dir)?,
            Some(log_dir.join("aviutl2.LOG"))
        );
        Ok(())
    }
}
