# aviutl2 CLI

AviUtl2のプラグイン・スクリプト開発に便利なコマンドラインツール。

## インストール

ビルド済みバイナリは[Releases](https://github.com/sevenc-nanashi/aviutl2-cli/releases/latest)からダウンロードできます：

```toml
# mise.toml の例
[tools]
"github:sevenc-nanashi/aviutl2-cli" = { version = "latest", bin = "au2.exe" }
```

crates.ioからもインストールできます：

```sh
cargo install aviutl2-cli
```

## 設定

設定は`aviutl2.toml`に記述します。`.config/aviutl2.toml`に配置することもできます。

<details>
<summary>aviutl2.toml の例</summary>

詳細な仕様は[TypeSpec](./typespec/main.tsp)を参照してください。

```toml
[project]
# プロジェクト名
name = "MyAviUtlPlugin"
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
# zipの名前（`.au2pkg.zip`は自動で付与されます）
zip_name = "{name}-v{version}"
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
# プラグイン名
name = "My AviUtl2 Plugin"
# 作者名
author = "Nanashi."
# 種類
# "common" / "effect" / "input" / "output" / "script" / "modification"
type = "common"
# パッケージのサイト
homepage = "https://github.com/sevenc-nanashi/tinted-aviutl2"
# ニコニ・コモンズID
niconi_commons_id = "sm45904404"
# タグ
tags = ["UI"]
# 概要
summary = "ぼくのかんがえたさいきょうのAviUtl2プラグイン"

# 詳細説明
description = { type = "url", url = "https://raw.githubusercontent.com/sevenc-nanashi/tinted-aviutl2/main/README.md" }

# ライセンス
license = { type = "MIT", template = false, text = { type = "file", path = "./LICENSE" } }

# ダウンロード元
download_source = { type = "github", owner = "sevenc-nanashi", repo = "tinted-aviutl2" }
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
