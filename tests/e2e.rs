use assert_cmd::Command;
use fs_err as fs;
use predicates::str::contains;
use std::{io::Read, path::Path};
use tempfile::tempdir;

const MIT_LICENSE_TEXT: &str = r#"MIT License

Copyright (c) 2026 Example

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"#;

fn write_file(path: &Path, content: &str) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

#[test]
fn e2e_init_creates_config_and_updates_gitignore() -> anyhow::Result<()> {
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
    assert!(gitignore.contains("/.aviutl2-cli"));
    assert!(gitignore.contains("/release"));

    Ok(())
}

#[test]
fn e2e_init_pkl_creates_pkl_config() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let project_dir = temp.path().join("my_pkl_project");
    fs::create_dir_all(&project_dir)?;

    Command::new(assert_cmd::cargo::cargo_bin!("au2"))
        .current_dir(&project_dir)
        .arg("init")
        .arg("--pkl")
        .assert()
        .success();

    let config_path = project_dir.join("aviutl2.pkl");
    let config = fs::read_to_string(&config_path)?;
    assert!(
        config.contains("amends \"https://raw.githubusercontent.com/sevenc-nanashi/aviutl2-cli")
    );
    assert!(config.contains("name = \"my_pkl_project\""));

    Ok(())
}

#[test]
fn e2e_init_fails_when_config_exists() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let project_dir = temp.path().join("existing_project");
    fs::create_dir_all(&project_dir)?;

    let config_path = project_dir.join("aviutl2.toml");
    write_file(
        &config_path,
        r#"[project]
            id = "sevenc-nanashi.aviutl2-cli.existing-project"
            name = "existing_project"
            version = "0.1.0"
            "#,
    )?;

    Command::new(assert_cmd::cargo::cargo_bin!("au2"))
        .current_dir(&project_dir)
        .arg("init")
        .assert()
        .failure()
        .stdout(contains("aviutl2.toml は既に存在します"));

    Ok(())
}

#[test]
fn e2e_prepare_schema_writes_schema_file() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let project_dir = temp.path().join("schema_project");
    fs::create_dir_all(&project_dir)?;

    let config_path = project_dir.join("aviutl2.toml");
    write_file(
        &config_path,
        dedent::dedent!(
            r#"[project]
               id = "sevenc-nanashi.aviutl2-cli.schema-project"
               name = "schema"
               version = "0.1.0"

               [artifacts]

               [development]
               aviutl2_version = "latest"
               install_dir = "devdir"
           "#
        ),
    )?;

    Command::new(assert_cmd::cargo::cargo_bin!("au2"))
        .current_dir(&project_dir)
        .arg("prepare:schema")
        .assert()
        .success();

    let schema_path = project_dir.join(".aviutl2-cli").join("aviutl2.schema.json");
    assert!(schema_path.exists());

    Ok(())
}

#[test]
fn e2e_release_writes_catalog_json() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let project_dir = temp.path().join("catalog_project");
    fs::create_dir_all(&project_dir)?;
    fs::create_dir_all(project_dir.join("dist"))?;
    write_file(
        &project_dir.join("dist").join("plugin.auf"),
        "dummy-binary-content",
    )?;

    let config_path = project_dir.join("aviutl2.toml");
    write_file(
        &config_path,
        dedent::dedent!(
            r#"[project]
               id = "sevenc-nanashi.aviutl2-cli.catalog-project"
               name = "catalog-project"
               version = "1.2.3"

               [artifacts.plugin]
               source = "dist/plugin.auf"
               destination = "Plugin/plugin.auf"
               placement_method = "copy"

               [catalog]
               id = "my-plugin"
               description_path = "README.md"
               license_path = "LICENSE"
               download_repo = { owner = "sevenc-nanashi", repo = "aviutl2-cli" }

               [release]
               "#
        ),
    )?;
    write_file(&project_dir.join("README.md"), "desc")?;
    write_file(&project_dir.join("LICENSE"), MIT_LICENSE_TEXT)?;

    Command::new(assert_cmd::cargo::cargo_bin!("au2"))
        .current_dir(&project_dir)
        .arg("release")
        .assert()
        .success();

    let catalog_json_path = project_dir.join("release").join("catalog.json");
    assert!(catalog_json_path.exists());
    let content = fs::read_to_string(&catalog_json_path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;
    assert_eq!(json[0]["id"], "my-plugin");
    assert_eq!(json[0]["description"], "desc");
    assert_eq!(json[0]["licenses"][0]["type"], "MIT");
    assert_eq!(json[0]["licenses"][0]["isCustom"], true);
    assert_eq!(json[0]["licenses"][0]["licenseBody"], MIT_LICENSE_TEXT);
    assert_eq!(
        json[0]["installer"]["source"]["github"]["repo"],
        "aviutl2-cli"
    );
    assert_eq!(
        json[0]["installer"]["source"]["github"]["pattern"],
        r"^sevenc-nanashi\.aviutl2-cli\.catalog-project-v[^/]+\.au2pkg\.zip$"
    );
    assert_eq!(json[0]["installer"]["install"][0]["action"], "download");
    assert_eq!(json[0]["installer"]["install"][1]["action"], "extract");
    assert_eq!(json[0]["installer"]["install"][2]["action"], "copy");
    assert_eq!(json[0]["version"][0]["version"], "1.2.3");
    assert_eq!(
        json[0]["version"][0]["file"][0]["path"],
        "{pluginsDir}/plugin.auf"
    );
    assert!(
        json[0]["version"][0]["file"][0]["XXH3_128"]
            .as_str()
            .unwrap_or_default()
            .len()
            == 32
    );

    Ok(())
}

#[test]
fn e2e_release_catalog_uses_explicit_license_type() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let project_dir = temp.path().join("catalog_explicit_license_project");
    fs::create_dir_all(&project_dir)?;
    fs::create_dir_all(project_dir.join("dist"))?;
    write_file(
        &project_dir.join("dist").join("plugin.auf"),
        "dummy-binary-content",
    )?;

    write_file(
        &project_dir.join("aviutl2.toml"),
        dedent::dedent!(
            r#"[project]
               id = "sevenc-nanashi.aviutl2-cli.catalog-explicit-license-project"
               name = "catalog-explicit-license-project"
               version = "1.2.3"

               [artifacts.plugin]
               source = "dist/plugin.auf"
               destination = "Plugin/plugin.auf"
               placement_method = "copy"

               [catalog]
               id = "my-plugin"
               license_path = { type = "Apache-2.0", path = "LICENSE" }

               [release]
               "#
        ),
    )?;
    write_file(&project_dir.join("LICENSE"), "not an apache text")?;

    Command::new(assert_cmd::cargo::cargo_bin!("au2"))
        .current_dir(&project_dir)
        .arg("release")
        .assert()
        .success();

    let catalog_json_path = project_dir.join("release").join("catalog.json");
    let content = fs::read_to_string(&catalog_json_path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;
    assert_eq!(json[0]["licenses"][0]["type"], "Apache-2.0");
    assert_eq!(json[0]["licenses"][0]["isCustom"], true);

    Ok(())
}

#[test]
fn e2e_release_catalog_warns_when_license_detection_falls_back_to_custom() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let project_dir = temp.path().join("catalog_custom_license_project");
    fs::create_dir_all(&project_dir)?;
    fs::create_dir_all(project_dir.join("dist"))?;
    write_file(
        &project_dir.join("dist").join("plugin.auf"),
        "dummy-binary-content",
    )?;

    write_file(
        &project_dir.join("aviutl2.toml"),
        dedent::dedent!(
            r#"[project]
               id = "sevenc-nanashi.aviutl2-cli.catalog-custom-license-project"
               name = "catalog-custom-license-project"
               version = "1.2.3"

               [artifacts.plugin]
               source = "dist/plugin.auf"
               destination = "Plugin/plugin.auf"
               placement_method = "copy"

               [catalog]
               id = "my-plugin"
               license_path = "LICENSE"

               [release]
               "#
        ),
    )?;
    write_file(&project_dir.join("LICENSE"), "my original license text")?;

    Command::new(assert_cmd::cargo::cargo_bin!("au2"))
        .current_dir(&project_dir)
        .arg("release")
        .assert()
        .success()
        .stdout(contains("ライセンスの種別の自動検出に失敗しました。"));

    let catalog_json_path = project_dir.join("release").join("catalog.json");
    let content = fs::read_to_string(&catalog_json_path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;
    assert_eq!(json[0]["licenses"][0]["type"], "カスタムライセンス");
    assert_eq!(json[0]["licenses"][0]["isCustom"], true);

    Ok(())
}

#[test]
fn e2e_release_catalog_omits_optional_keys_when_paths_are_missing() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let project_dir = temp.path().join("catalog_omit_project");
    fs::create_dir_all(&project_dir)?;
    fs::create_dir_all(project_dir.join("dist"))?;
    write_file(
        &project_dir.join("dist").join("plugin.auf"),
        "dummy-binary-content",
    )?;

    write_file(
        &project_dir.join("aviutl2.toml"),
        dedent::dedent!(
            r#"[project]
               id = "sevenc-nanashi.aviutl2-cli.catalog-omit-project"
               name = "catalog-omit-project"
               version = "1.2.3"

               [artifacts.plugin]
               source = "dist/plugin.auf"
               destination = "Plugin/plugin.auf"
               placement_method = "copy"

               [catalog]
               id = "my-plugin"

               [release]
               "#
        ),
    )?;

    Command::new(assert_cmd::cargo::cargo_bin!("au2"))
        .current_dir(&project_dir)
        .arg("release")
        .assert()
        .success();

    let catalog_json_path = project_dir.join("release").join("catalog.json");
    let content = fs::read_to_string(&catalog_json_path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;
    assert!(json[0].get("description").is_none());
    assert!(json[0].get("licenses").is_none());
    assert!(json[0]["installer"].get("source").is_none());
    assert!(json[0]["installer"].get("install").is_some());
    assert!(json[0]["installer"].get("uninstall").is_some());

    Ok(())
}

#[test]
fn e2e_release_fails_when_catalog_path_is_invalid() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let project_dir = temp.path().join("catalog_invalid_path_project");
    fs::create_dir_all(&project_dir)?;
    fs::create_dir_all(project_dir.join("dist"))?;
    write_file(
        &project_dir.join("dist").join("plugin.auf"),
        "dummy-binary-content",
    )?;

    write_file(
        &project_dir.join("aviutl2.toml"),
        dedent::dedent!(
            r#"[project]
               id = "sevenc-nanashi.aviutl2-cli.catalog-invalid-path-project"
               name = "catalog-invalid-path-project"
               version = "1.2.3"

               [artifacts.plugin]
               source = "dist/plugin.auf"
               destination = "Plugin/plugin.auf"
               placement_method = "copy"

               [catalog]
               id = "my-plugin"
               license_path = "MISSING_LICENSE"

               [release]
               "#
        ),
    )?;

    Command::new(assert_cmd::cargo::cargo_bin!("au2"))
        .current_dir(&project_dir)
        .arg("release")
        .assert()
        .failure()
        .stdout(contains("ライセンスファイルの読み込みに失敗しました"));

    Ok(())
}

#[test]
fn e2e_config_patch_overrides_version() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let project_dir = temp.path().join("config_patch_project");
    fs::create_dir_all(&project_dir)?;
    fs::create_dir_all(project_dir.join("dist"))?;
    write_file(
        &project_dir.join("dist").join("plugin.auf"),
        "dummy-binary-content",
    )?;

    write_file(
        &project_dir.join("aviutl2.toml"),
        dedent::dedent!(
            r#"[project]
               id = "sevenc-nanashi.aviutl2-cli.config-patch-project"
               name = "config-patch-project"
               version = "0.1.0"

               [artifacts.plugin]
               source = "dist/plugin.auf"
               destination = "Plugin/plugin.auf"
               placement_method = "copy"

               [release]
               "#
        ),
    )?;

    write_file(
        &project_dir.join("patch.toml"),
        dedent::dedent!(
            r#"[project]
               version = "9.9.9"
               "#
        ),
    )?;

    Command::new(assert_cmd::cargo::cargo_bin!("au2"))
        .current_dir(&project_dir)
        .args(["-c", "patch.toml", "release"])
        .assert()
        .success();

    let zip_path = project_dir
        .join("release")
        .join("sevenc-nanashi.aviutl2-cli.config-patch-project-v9.9.9.au2pkg.zip");
    assert!(zip_path.exists(), "パッチしたバージョンのzipが存在するはず");

    let old_zip_path = project_dir
        .join("release")
        .join("sevenc-nanashi.aviutl2-cli.config-patch-project-v0.1.0.au2pkg.zip");
    assert!(
        !old_zip_path.exists(),
        "元のバージョンのzipは存在しないはず"
    );

    Ok(())
}

#[test]
fn e2e_config_override_uses_other_file() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let project_dir = temp.path().join("config_override_project");
    fs::create_dir_all(&project_dir)?;

    let config_dir = temp.path().join("config_dir");
    fs::create_dir_all(&config_dir)?;
    fs::create_dir_all(config_dir.join("dist"))?;
    write_file(
        &config_dir.join("dist").join("plugin.auf"),
        "dummy-binary-content",
    )?;

    write_file(
        &config_dir.join("override.toml"),
        dedent::dedent!(
            r#"[project]
               id = "sevenc-nanashi.aviutl2-cli.config-override-project"
               name = "config-override-project"
               version = "2.0.0"

               [artifacts.plugin]
               source = "dist/plugin.auf"
               destination = "Plugin/plugin.auf"
               placement_method = "copy"

               [release]
               "#
        ),
    )?;

    let override_path = config_dir.join("override.toml");
    Command::new(assert_cmd::cargo::cargo_bin!("au2"))
        .current_dir(&project_dir)
        .args(["-C", override_path.to_str().unwrap(), "release"])
        .assert()
        .success();

    // リリース先はconfig_dir（上書きファイルのディレクトリ）を基準にする
    let zip_path = config_dir
        .join("release")
        .join("sevenc-nanashi.aviutl2-cli.config-override-project-v2.0.0.au2pkg.zip");
    assert!(zip_path.exists(), "config_dirにzipが作成されるはず");

    Ok(())
}

#[test]
fn e2e_release_creates_package_information() -> anyhow::Result<()> {
    let temp = tempdir()?;
    let project_dir = temp.path().join("package_info_project");
    fs::create_dir_all(&project_dir)?;
    fs::create_dir_all(project_dir.join("dist"))?;
    write_file(
        &project_dir.join("dist").join("plugin.auf"),
        "dummy-binary-content",
    )?;

    let config_path = project_dir.join("aviutl2.toml");
    write_file(
        &config_path,
        dedent::dedent!(
            r#"[project]
               id = "sevenc-nanashi.aviutl2-cli.package-info-project"
               name = "package-info-project"
               version = "0.1.0"

               [artifacts.plugin]
               source = "dist/plugin.auf"
               destination = "Plugin/plugin.auf"
               placement_method = "copy"

               [release]
               package_template = "./package_template.txt"
           "#
        ),
    )?;

    write_file(
        &project_dir.join("package_template.txt"),
        "This is a package for {name} version {version}.",
    )?;

    Command::new(assert_cmd::cargo::cargo_bin!("au2"))
        .current_dir(&project_dir)
        .arg("release")
        .assert()
        .success();

    let zip_path = project_dir
        .join("release")
        .join("sevenc-nanashi.aviutl2-cli.package-info-project-v0.1.0.au2pkg.zip");
    assert!(zip_path.exists());
    let mut zip = zip::ZipArchive::new(std::fs::File::open(zip_path)?)?;

    let mut file = zip.by_name("package.txt")?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    assert_eq!(
        content,
        "This is a package for package-info-project version 0.1.0."
    );
    drop(file);

    let mut ini_file = zip.by_name("package.ini")?;
    let mut ini_content = String::new();
    ini_file.read_to_string(&mut ini_content)?;
    let mut ini = configparser::ini::Ini::new();
    ini.read(ini_content).unwrap();
    assert_eq!(
        ini.get_map_ref()
            .get("package")
            .and_then(|s| s.get("id").cloned())
            .flatten(),
        Some("sevenc-nanashi.aviutl2-cli.package-info-project".to_string())
    );
    assert_eq!(
        ini.get_map_ref()
            .get("package")
            .and_then(|s| s.get("information").cloned())
            .flatten(),
        Some("package-info-project v0.1.0".to_string())
    );

    Ok(())
}
