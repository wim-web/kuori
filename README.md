## config

### validate

```
kuori validate --config config.example.json
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
