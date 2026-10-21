# kuori

`kuori` は、JSON で定義したタスクを使って、SSH 経由でリモートホストにスクリプトを配布・実行する CLI ツールです。  
「同じ運用コマンドを複数ホストへ安全に順次実行したい」用途を想定しています。

## 何をするツールか

- 設定ファイル（JSON）からタスク一覧を読み込む
- `~/.ssh/config` を使って対象ホストへ接続する
- ローカルの `script_path` をリモートへ転送して実行する
- 実行時に環境変数 (`environments`) を付与できる
- `sudo` 指定があれば `sudo bash ...` で実行する
- 実行後、転送した一時スクリプトを削除する

## 事前準備

- ローカルに `kuori` をインストール済みであること
- `~/.ssh/config` に `host` で参照するエントリがあること
  - 少なくとも `HostName` / `User` / `IdentityFile` が解決できること
- 各タスクの `script_path` がローカルで存在すること
- 各タスクの `working_dir` がリモート上で存在し、書き込み可能であること

## クイックスタート

```bash
# 1) 設定ファイルを検証
kuori validate --config config.example.json

# 2) 全タスク実行
kuori run --config config.example.json

# 3) 一部タスクのみ実行（name をカンマ区切り）
kuori run --config config.example.json --task-names deploy-api,restart-worker
```

後方互換として、サブコマンドなしでも `run` 相当で実行できます。

```bash
kuori --config config.example.json --task-names deploy-api
```

## 設定ファイル（JSON）

`config.example.json`:

```json
{
  "tasks": [
    {
      "name": "deploy-api",
      "host": "api-server",
      "script_path": "./scripts/deploy-api.sh",
      "working_dir": "/tmp",
      "sudo": false,
      "environments": {
        "LOG_LEVEL": "info",
        "APP_ENV": "production"
      }
    }
  ]
}
```

### フィールド定義

| フィールド | 型 | 必須 | 説明 |
| --- | --- | --- | --- |
| `tasks` | array | はい | 実行タスクの配列（1件以上必須） |
| `tasks[].name` | string | はい | タスク識別名。`task_names` 指定時に使う。重複不可 |
| `tasks[].host` | string | はい | `~/.ssh/config` の `Host` 名 |
| `tasks[].script_path` | string | はい | ローカルの実行スクリプトパス |
| `tasks[].working_dir` | string | はい | リモート側の作業ディレクトリ |
| `tasks[].sudo` | bool | はい | `true` なら `sudo bash` で実行 |
| `tasks[].environments` | object | はい | 追加環境変数（`KEY: VALUE`） |

### バリデーション内容

`kuori validate`（および `kuori run` の内部）で以下を検証します。

- `tasks` が空でないこと
- 各 `task` の `name` / `host` / `script_path` / `working_dir` が空文字でないこと
- `task.name` が重複していないこと
- `task.timeout_sec` を指定した場合に 1 以上であること

## コマンド一覧

### `run`

設定ファイルを読み込み、タスクを順番に実行します。

```bash
kuori run --config config.json
kuori run --config config.json --task-names deploy-api,restart-worker
kuori run --config config.json --dry-run
```

`--dry-run` を付けると、SSH接続せずに実行予定タスクを表示します。

`timeout_sec`（秒）と `retry`（再試行回数）を `task` ごとに指定できます。

- `timeout_sec`: 1回の実行で許可する最大秒数（未指定で無制限）
- `retry`: 失敗時の再試行回数（`0` で再試行なし）

`example` を使う場合:

```bash
kuori validate --config config.example.json
kuori run --config config.example.json
kuori run --config config.example.json --dry-run
```

### `validate`

設定ファイルを検証のみ行います（実行なし）。

```bash
kuori validate --config config.json
```

### `update`

`kuori` 自身を最新へ更新します。GitHub Releases の最新バイナリをダウンロードして置き換えます。

```bash
kuori update
```

### `--version`

バージョン情報（`semver+sha`）を表示します。

```bash
kuori --version
```

## 補足

- `--task-names` は完全一致です（部分一致しません）
- タスクは並列ではなく順次実行です
- 標準出力はリモート実行結果をそのままストリーム表示します

## ビルドとリリース

CI のテストとリリースは、`go.mod` の `go` 行で指定した Go 1.26.10 を使用します。
ローカルでも同じツールチェーンで検証・ビルドできます。

```bash
GOTOOLCHAIN=go1.26.10 go test ./...
GOTOOLCHAIN=go1.26.10 go build -o kuori .
go version -m ./kuori
```

### eMudhra 中間CA秘密鍵漏えいへの対応（2026-10-21）

`kuori update` は HTTPS を使うため、[Go 公式のセキュリティ告知](https://groups.google.com/g/golang-announce/c/66pojueaQek)
に従い、漏えいした中間CAを拒否する Go 1.26.10（または 1.27 系では 1.27.3 以降）で再ビルドする必要があります。
`go.mod` の `go` 行は最低要件であり、既存の実行ファイルは更新しません。別のツールチェーンを使う場合も対策版であることを確認してください。

配布済み [v2.0.5](https://github.com/wim-web/kuori/releases/tag/v2.0.5) の Linux amd64 と macOS arm64 の両バイナリは Go 1.26.1 製です。修正のマージ後、次の作業が必要です。

1. 修正コミットから新しいバージョンのタグを作成し、リリースワークフローで Linux amd64 と macOS arm64 のバイナリを再ビルドする。
2. ワークフローの `Verify binary Go version` で、両方の配布物のビルド情報が指定した Go バージョンと一致することを確認する。
3. 新リリースを公開し、既存インストールを対策済みバイナリへ置き換える。初回の置換は、更新済みのブラウザ等で GitHub Releases から直接取得して行う。

古い `kuori update` 自体も未対策の Go で HTTPS 通信するため、初回の更新手段には使用しないでください。
インストール先の OS・CA ストアも最新にし、証明書検証は有効のまま使用してください。
