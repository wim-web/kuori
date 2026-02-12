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
        "RUST_LOG": "info",
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

## コマンド一覧

### `run`

設定ファイルを読み込み、タスクを順番に実行します。

```bash
kuori run --config config.json
kuori run --config config.json --task-names deploy-api,restart-worker
```

### `validate`

設定ファイルを検証のみ行います（実行なし）。

```bash
kuori validate --config config.json
```

### `update`

`kuori` 自身を最新へ更新します。

- macOS: `cargo install --git` で更新
- Linux (`x86_64-unknown-linux-gnu`): GitHub Releases の最新バイナリで更新

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
