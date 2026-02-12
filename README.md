## config

### example

`/Users/wim/.codex/worktrees/602e/kuori/config.example.json`:

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
    },
    {
      "name": "restart-worker",
      "host": "worker-server",
      "script_path": "./scripts/restart-worker.sh",
      "working_dir": "/tmp",
      "sudo": true,
      "environments": {}
    }
  ]
}
```

### validate

```
kuori validate --config config.json
```

`kuori` 実行時に、以下を自動で検証します。

- `tasks` が空でないこと
- 各 `task` の `name` / `host` / `script_path` / `working_dir` が空文字でないこと
- `task.name` が重複していないこと

### run

```
kuori run --config config.json
```

`example` を使う場合:

```bash
kuori validate --config config.example.json
kuori run --config config.example.json
```
