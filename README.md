# aviutl2 CLI

AviUtl2のプラグイン・スクリプト開発に便利なコマンドラインツール。

## 設定

設定は`aviutl2.toml`に記述します。`.config/aviutl2.toml`に配置することもできます。

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

# 開発時の設定
[development]
# ダウンロードするAviUtl2のバージョン
aviutl2_version = "2.00beta31"
# AviUtl2のインストール先ディレクトリ
install_dir = "./development"

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

# 含める成果物のリスト（省略時はすべて含める）
include = ["my_plugin_aul2", "my_plugin_aux2"]
```

## コマンド一覧

### `au2 init`

`aviutl2.toml`を作成します。

### `au2 prepare`

AviUtl2の開発環境をセットアップします（`prepare:schema -> prepare:aviutl2 -> prepare:artifacts`）。

### `au2 prepare:schema`

設定ファイルのJSON Schemaを開発用ディレクトリに出力します。

### `au2 prepare:aviutl2`

AviUtl2本体をダウンロードし、開発用ディレクトリに展開します。

### `au2 prepare:artifacts`

開発用ディレクトリに成果物へのシンボリックリンクを作成します。

### `au2 develop` / `au2 dev`

開発用の成果物をビルドし、AviUtl2に配置します。

### `au2 release`

成果物をビルドし、リリース用のパッケージを作成します。
`--set-version` を指定すると `aviutl2.toml` の `project.version` を上書きできます。

## ライセンス

MIT License で公開しています。
