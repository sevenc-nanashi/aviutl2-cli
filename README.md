# aviutl2 CLI

AviUtl2のプラグイン・スクリプト開発に便利なコマンドラインツール。

> [!WARNING]
> このツールはデフォルトではシンボリックリンクを使用してファイルを配置します。
> そのため、Windowsの開発者モードを有効にする必要があります。

## インストール

ビルド済みバイナリは[Releases](https://github.com/sevenc-nanashi/aviutl2-cli/releases/latest)からダウンロードできます：

```toml
# mise.toml の例
[tools]
"github:sevenc-nanashi/aviutl2-cli" = { version = "latest", bin = "au2.exe" }
```

crates.ioからもインストールできます：

```sh
# cargo-binstallを使う場合
cargo binstall aviutl2-cli

# ソースからビルドする場合
cargo install aviutl2-cli
```

## 設定

以下の設定フォーマットをサポートしています：

- [`pkl`](https://pkl-lang.org)
- TOML
- YAML
- JSON

以下のファイル名でプロジェクトルートに配置できます（優先順位の高い順）：

1. `aviutl2.{pkl,toml,yaml,json}`
2. `.aviutl2.{pkl,toml,yaml,json}`
3. `.aviutl2-cli/aviutl2.{pkl,toml,yaml,json}`

> [!NOTE]
> デフォルトではpklは[pklr](https://github.com/jdx/pklr)を使用して処理されます。
> `AU2_USE_PKL_CLI` 環境変数を設定すると、システムの `pkl` コマンドが使用されます。

<details>
<summary>aviutl2.toml の例</summary>

詳細な仕様は[AviUtl2CliConfig.pkl](./src/AviUtl2CliConfig.pkl)を参照してください。

```toml
[project]
# プロジェクトID
id = "my-aviutl2-plugin"
# プロジェクト名
name = "MyAviUtl2Plugin"
# バージョン
version = "0.1.0"

# 成果物の設定
[artifacts.my_plugin_aul2]
# 成果物のファイルパス
source = "i18n/english.aul2"
# http/https の URL も指定できます
# source = "https://example.com/my_plugin.aul2"
# 成果物の有効/無効（デフォルトは true）
enabled = true
# AviUtlのプラグインディレクトリ内での配置先パス
destination = "Language/English.my_plugin.aul2"
# ビルドコマンド
build = "ruby ./scripts/build_aul2.rb"
# 開発時の配置方法（symlink / copy、デフォルトはsymlink）
placement_method = "symlink"

[artifacts.my_plugin_aux2]
destination = "Plugin/my_plugin.aux2"

# プロファイルごとのビルド設定
[artifacts.my_plugin_aux2.profiles.debug]
build = "cargo build"
source = "target/debug/my_plugin_aux2.dll"

[artifacts.my_plugin_aux2.profiles.release]
# buildコマンドは複数も指定可能（前から順に実行される）
build = ["echo Building release...", "cargo build --release"]
source = "target/release/my_plugin_aux2.dll"
enabled = true

# ビルドグループの定義
# 1つのコマンドが複数の成果物をビルドする場合に使います。
[build_group]
hoge = ["cmake -S . -B build", "cmake --build build"]

[artifacts.my_plugin_tool]
source = "target/release/my_plugin_tool.dll"
destination = "Plugin/my_plugin_tool.auf2"
build = { group = "hoge" }

[artifacts.my_plugin_tool_2]
source = "target/release/my_plugin_tool_2.dll"
destination = "Plugin/my_plugin_tool_2.auf2"
build = { group = "hoge" }

# 開発時の設定
[development]
# ダウンロードするAviUtl2のバージョン
aviutl2_version = "latest"
# AviUtl2のインストール先ディレクトリ（省略時は ./.aviutl2-cli/development）
install_dir = "./.aviutl2-cli/development"
# 開発用の事前/事後ビルドコマンド
prebuild = "echo prebuild"
postbuild = "echo postbuild"

# プレビュー用の設定
[preview]
# ダウンロードするAviUtl2のバージョン（省略時は development.aviutl2_version）
aviutl2_version = "latest"
# AviUtl2のインストール先ディレクトリ（省略時は ./.aviutl2-cli/preview）
install_dir = "./.aviutl2-cli/preview"
# 使うプロファイル（デフォルトは`release`）
profile = "release"
# 含める成果物のリスト（省略時は release.include を使用）
include = ["my_plugin_aul2", "my_plugin_aux2"]
# プレビュー用の事前/事後ビルドコマンド
prebuild = "echo prebuild"
postbuild = "echo postbuild"

# リリース設定
[release]
# 出力ディレクトリ
output_dir = "release"
# package.txtのテンプレートファイル（オプション）
package_template = "package_template.txt"
# package.iniでのID（デフォルトは`{id}`）
package_id = "my-plugin"
# package.iniでの名前（デフォルトは`{name}`）
package_name = "My Plugin"
# package.iniでの情報欄（デフォルトは`{name} v{version}`）
package_information = "{name} v{version}"
# zipの名前（`.au2pkg.zip`は自動で付与されます）
zip_name = "{id}-v{version}"
# 使うプロファイル（デフォルトは`release`）
profile = "release"
# リリース用の事前/事後ビルドコマンド
prebuild = "echo prebuild"
postbuild = "echo postbuild"

# 含める成果物のリスト（省略時はすべて含める）
include = ["my_plugin_aul2", "my_plugin_aux2"]

# AviUtl2 カタログ用の情報
# JSON入力で使えるデータを指定します。
[catalog]
# プラグインID
id = "sevenc-nanashi.my-plugin"
# 説明文（Markdown）へのパス
description_path = "./README.md"
# ライセンス
# 文字列指定時は本文から自動検出
license_path = "./LICENSE"
# 明示指定も可能
# license_path = { type = "MIT", path = "./LICENSE" }
# ダウンロード元のリポジトリ
download_repo = { owner = "sevenc-nanashi", repo = "tinted-aviutl2" }
```

</details>

## コマンド一覧

### `au2 init`

`aviutl2.toml`を作成します。

### `au2 prepare`

AviUtl2の開発環境をセットアップします（`prepare:schema -> prepare:aviutl2 -> prepare:artifacts`）。
HTTP の成果物は `.aviutl2-cli/cache` にキャッシュされ、再取得する場合は `--refresh` を指定します。

### `au2 prepare:schema`

設定ファイルのJSON Schemaを開発用ディレクトリに出力します。

### `au2 prepare:aviutl2`

AviUtl2本体をダウンロードし、開発用ディレクトリに展開します。

### `au2 prepare:artifacts`

開発用ディレクトリに成果物へのシンボリックリンクを作成します。
HTTP の成果物を再取得する場合は `--refresh` を指定します。

### `au2 develop` / `au2 dev`

開発用の成果物をビルドし、AviUtl2に配置します。
HTTP の成果物を再取得する場合は `--refresh` を指定します。
AviUtl2の起動後は開発用ディレクトリの `data/log` にある最新のログを追尾します。
ログを追尾せずに終了する場合は `--detach` を指定します。

### `au2 release`

成果物をビルドし、リリース用のパッケージを作成します。
`--set-version` を指定すると `aviutl2.toml` の `project.version` を上書きできます。

### `au2 preview`

リリース用の成果物をビルドし、プレビュー用ディレクトリに配置します。

## TypeSpec

設定ファイルの JSON Schema は TypeSpec から生成しています。

```sh
nr typespec
```

生成物:

- `typespec/temporary/aviutl2.config.schema.json`（TypeSpec の出力）
- `src/schema.json`（CLI が参照する最終的な schema）

## ライセンス

MIT License で公開しています。
