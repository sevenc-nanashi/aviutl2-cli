use assert_cmd::Command;
use fs_err as fs;
use predicates::str::contains;
use std::path::Path;
use tempfile::tempdir;

fn write_file(path: &Path, content: &str) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

#[test]
fn e2e_init_creates_config_and_updates_gitignore() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let project_dir = temp.path().join("my_aviutl2_project");
    fs::create_dir_all(&project_dir)?;

    let gitignore_path = project_dir.join(".gitignore");
    write_file(&gitignore_path, "target\n")?;

    Command::new(assert_cmd::cargo::cargo_bin!("au2"))
        .current_dir(&project_dir)
        .arg("init")
        .assert()
        .success();

    let config_path = project_dir.join("aviutl2.toml");
    let config = fs::read_to_string(&config_path)?;
    assert!(config.contains("name = \"my_aviutl2_project\""));

    let gitignore = fs::read_to_string(&gitignore_path)?;
    assert!(gitignore.contains("/development"));
    assert!(gitignore.contains("/release"));

    Ok(())
}

#[test]
fn e2e_init_fails_when_config_exists() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let project_dir = temp.path().join("existing_project");
    fs::create_dir_all(&project_dir)?;

    let config_path = project_dir.join("aviutl2.toml");
    write_file(
        &config_path,
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[artifacts]\n",
    )?;

    Command::new(assert_cmd::cargo::cargo_bin!("au2"))
        .current_dir(&project_dir)
        .arg("init")
        .assert()
        .failure()
        .stderr(contains("aviutl2.toml は既に存在します"));

    Ok(())
}

#[test]
fn e2e_prepare_schema_writes_schema_file() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let project_dir = temp.path().join("schema_project");
    fs::create_dir_all(&project_dir)?;

    let config_path = project_dir.join("aviutl2.toml");
    write_file(
        &config_path,
        r#"[project]
name = "schema"
version = "0.1.0"

[artifacts]

[development]
aviutl2_version = "2.00beta31"
install_dir = "devdir"
"#,
    )?;

    Command::new(assert_cmd::cargo::cargo_bin!("au2"))
        .current_dir(&project_dir)
        .arg("prepare:schema")
        .assert()
        .success();

    let schema_path = project_dir.join("devdir").join("aviutl2.schema.json");
    assert!(schema_path.exists());

    Ok(())
}
