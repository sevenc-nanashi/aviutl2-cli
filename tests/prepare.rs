use assert_cmd::Command;
use fs_err as fs;
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command as ProcessCommand, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

fn write_file(path: &Path, content: &[u8]) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

fn can_create_symlink() -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::symlink_file;
        let temp = match tempdir() {
            Ok(temp) => temp,
            Err(_) => return false,
        };
        let source = temp.path().join("source.txt");
        let dest = temp.path().join("dest.txt");
        if write_file(&source, b"probe").is_err() {
            return false;
        }
        match symlink_file(&source, &dest) {
            Ok(_) => true,
            Err(err) => err.kind() != std::io::ErrorKind::PermissionDenied,
        }
    }
    #[cfg(not(windows))]
    {
        true
    }
}

fn can_run_bun_serve() -> bool {
    let status = ProcessCommand::new("bun")
        .args(["run", "serve", "--", "--version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    matches!(status, Ok(status) if status.success())
}

fn find_free_port() -> Result<u16, std::io::Error> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    Ok(port)
}

fn wait_for_port(port: u16) -> bool {
    for _ in 0..40 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    false
}

struct ServerGuard {
    child: Child,
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn_server(root: &Path, port: u16) -> Result<ServerGuard, Box<dyn std::error::Error>> {
    let child = ProcessCommand::new("bun")
        .args([
            "run",
            "serve",
            "--",
            "--listen",
            &port.to_string(),
            root.to_string_lossy().as_ref(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(ServerGuard { child })
}

#[test]
fn prepare_artifacts_copies_file_to_data_dir() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let project_dir = temp.path().join("prepare_project");
    fs::create_dir_all(&project_dir)?;

    let install_dir = project_dir.join("dev");
    let aviutl_dir = install_dir.join("app");
    let aviutl_exe = aviutl_dir.join("aviutl2.exe");
    write_file(&aviutl_exe, b"")?;

    let source_path = project_dir.join("artifacts").join("my_plugin.aux2");
    write_file(&source_path, b"dummy")?;

    let config_path = project_dir.join("aviutl2.toml");
    write_file(
        &config_path,
        br#"[project]
name = "prepare"
version = "0.1.0"

[artifacts.my_plugin]
source = "artifacts/my_plugin.aux2"
destination = "Plugin/my_plugin.aux2"
placement_method = "copy"

[development]
aviutl2_version = "2.00beta31"
install_dir = "dev"
"#,
    )?;

    Command::new(assert_cmd::cargo::cargo_bin!("au2"))
        .current_dir(&project_dir)
        .arg("prepare:artifacts")
        .arg("--force")
        .assert()
        .success();

    let copied = aviutl_dir
        .join("data")
        .join("Plugin")
        .join("my_plugin.aux2");
    assert!(copied.exists());

    Ok(())
}

#[test]
fn prepare_artifacts_creates_symlink_when_allowed() -> Result<(), Box<dyn std::error::Error>> {
    if !can_create_symlink() {
        eprintln!("symlink を作成できない環境のためスキップします");
        return Ok(());
    }

    let temp = tempdir()?;
    let project_dir = temp.path().join("prepare_project_symlink");
    fs::create_dir_all(&project_dir)?;

    let install_dir = project_dir.join("dev");
    let aviutl_dir = install_dir.join("app");
    let aviutl_exe = aviutl_dir.join("aviutl2.exe");
    write_file(&aviutl_exe, b"")?;

    let source_path = project_dir.join("artifacts").join("my_plugin.aux2");
    write_file(&source_path, b"dummy")?;

    let config_path = project_dir.join("aviutl2.toml");
    write_file(
        &config_path,
        br#"[project]
name = "prepare"
version = "0.1.0"

[artifacts.my_plugin]
source = "artifacts/my_plugin.aux2"
destination = "Plugin/my_plugin.aux2"
placement_method = "symlink"

[development]
aviutl2_version = "2.00beta31"
install_dir = "dev"
"#,
    )?;

    Command::new(assert_cmd::cargo::cargo_bin!("au2"))
        .current_dir(&project_dir)
        .arg("prepare:artifacts")
        .arg("--force")
        .assert()
        .success();

    let linked = aviutl_dir
        .join("data")
        .join("Plugin")
        .join("my_plugin.aux2");
    let metadata = fs::symlink_metadata(&linked)?;
    assert!(metadata.file_type().is_symlink());

    Ok(())
}

#[test]
fn prepare_artifacts_downloads_http_source() -> Result<(), Box<dyn std::error::Error>> {
    if !can_run_bun_serve() {
        eprintln!("bun run serve が利用できないためスキップします");
        return Ok(());
    }

    let temp = tempdir()?;
    let project_dir = temp.path().join("prepare_http_project");
    fs::create_dir_all(&project_dir)?;

    let server_root = temp.path().join("server");
    let source_name = "my_plugin.aux2";
    let source_path = server_root.join(source_name);
    write_file(&source_path, b"downloaded")?;

    let port = find_free_port()?;
    let _server = spawn_server(&server_root, port)?;
    if !wait_for_port(port) {
        return Err("http server did not start in time".into());
    }

    let install_dir = project_dir.join("dev");
    let aviutl_dir = install_dir.join("app");
    let aviutl_exe = aviutl_dir.join("aviutl2.exe");
    write_file(&aviutl_exe, b"")?;

    let config_path = project_dir.join("aviutl2.toml");
    let source_url = format!("http://127.0.0.1:{}/{}", port, source_name);
    write_file(
        &config_path,
        format!(
            "[project]\nname = \"prepare\"\nversion = \"0.1.0\"\n\n[artifacts.my_plugin]\nsource = \"{}\"\ndestination = \"Plugin/my_plugin.aux2\"\nplacement_method = \"copy\"\n\n[development]\naviutl2_version = \"2.00beta31\"\ninstall_dir = \"dev\"\n",
            source_url
        )
        .as_bytes(),
    )?;

    Command::new(assert_cmd::cargo::cargo_bin!("au2"))
        .current_dir(&project_dir)
        .arg("prepare:artifacts")
        .arg("--force")
        .assert()
        .success();

    let copied = aviutl_dir
        .join("data")
        .join("Plugin")
        .join("my_plugin.aux2");
    assert!(copied.exists());
    assert_eq!(fs::read(&copied)?, b"downloaded");

    Ok(())
}
