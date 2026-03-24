mod config;
mod ssh;
mod ssh_config;
mod update;
mod util;

use anyhow::Context;
use clap::{Args, Parser, Subcommand};
use config::Config;
use ssh::{KuoriClient, SessionManager};
use ssh_config::{read_ssh_config, SshConfigPath};
use std::{
    fs,
    path::{Path, PathBuf},
};

// コマンドライン引数
#[derive(Parser, Debug)]
#[command(
    name = "kuori",
    disable_version_flag = true,
    about = "SSH経由で複数ホストへスクリプトを配布・実行するCLIタスクランナー",
    long_about = "kuori は JSON で定義したタスクを順番に実行する CLI です。\n\
各タスクは `host` と `script_path` を持ち、指定したホストへスクリプトを転送して実行します。\n\
`run` 実行時には設定ファイルのバリデーションも自動で行われます。",
    after_help = "例:\n\
  kuori validate --config config.json\n\
  kuori run --config config.json\n\
  kuori run --config config.json --task-names deploy-api,restart-worker\n\
  kuori --config config.json  # 後方互換: `run` として実行\n"
)]
struct CliArgs {
    #[arg(
        short = 'v',
        long = "version",
        action = clap::ArgAction::SetTrue,
        help = "バージョン情報を表示して終了"
    )]
    version: bool,

    #[command(subcommand)]
    command: Option<CliCommand>,

    // 後方互換: `kuori --config ...` は `run` として扱う
    #[arg(
        short,
        long,
        value_name = "FILE",
        help = "設定ファイル(JSON)のパス（サブコマンド未指定時のみ有効）",
        long_help = "設定ファイル(JSON)のパス。\n\
サブコマンド未指定時は `run --config <FILE>` と同じ動作になります。"
    )]
    config: Option<PathBuf>, // --config で設定ファイルを指定
    #[arg(
        long,
        value_name = "NAMES",
        help = "実行対象 task.name をカンマ区切りで指定（サブコマンド未指定時のみ有効）",
        long_help = "実行対象の task.name をカンマ区切りで指定します。\n\
例: --task-names deploy-api,restart-worker\n\
サブコマンド未指定時は `run --task-names <NAMES>` と同じ動作になります。"
    )]
    task_names: Option<String>,
    #[arg(long)]
    dry_run: bool,
}

#[derive(Subcommand, Debug)]
enum CliCommand {
    #[command(
        about = "設定ファイルに定義されたタスクを実行",
        long_about = "設定ファイルを読み込み、バリデーションに成功したタスクを順番に実行します。\n\
`--task-names` を指定すると対象タスクを絞り込めます。"
    )]
    Run(RunArgs),
    #[command(
        about = "設定ファイルの妥当性のみ検証",
        long_about = "設定ファイル(JSON)の形式と必須項目を検証します。\n\
実行は行わず、問題がある場合のみエラーで終了します。"
    )]
    Validate(ValidateArgs),
    #[command(
        about = "kuori を最新バージョンへ更新",
        long_about = "実行環境に応じて kuori を自己更新します。\n\
macOS: `cargo install --git` を利用\n\
linux/x86_64/gnu: GitHub Releases の最新バイナリを利用"
    )]
    Update,
}

#[derive(Args, Debug)]
struct RunArgs {
    #[arg(
        short,
        long,
        value_name = "FILE",
        help = "設定ファイル(JSON)のパス"
    )]
    config: PathBuf,
    #[arg(
        long,
        value_name = "NAMES",
        help = "実行対象 task.name をカンマ区切りで指定",
        long_help = "実行対象の task.name をカンマ区切りで指定します。\n\
例: --task-names deploy-api,restart-worker"
    )]
    task_names: Option<String>,
    #[arg(long)]
    dry_run: bool,
}

#[derive(Args, Debug)]
struct ValidateArgs {
    #[arg(
        short,
        long,
        value_name = "FILE",
        help = "検証対象の設定ファイル(JSON)のパス"
    )]
    config: PathBuf,
}

fn load_config(config_path: &PathBuf) -> anyhow::Result<Config> {
    let config_str = fs::read_to_string(config_path)
        .with_context(|| format!("failed to read config file: {}", config_path.display()))?;
    let mut config: Config = serde_json::from_str(&config_str)
        .with_context(|| format!("failed to parse config file: {}", config_path.display()))?;
    config
        .validate()
        .with_context(|| format!("invalid config file: {}", config_path.display()))?;

    // script_path を設定ファイルのディレクトリからの相対パスとして解決
    if let Some(config_dir) = config_path.parent() {
        for task in &mut config.tasks {
            let script_path = Path::new(&task.script_path);
            if script_path.is_relative() {
                task.script_path = config_dir
                    .join(script_path)
                    .to_string_lossy()
                    .to_string();
            }
        }
    }

    Ok(config)
}

fn parse_task_names(task_names: Option<String>) -> Option<Vec<String>> {
    task_names.map(|names| {
        names
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect::<Vec<String>>()
    })
}

fn format_env_keys(task: &config::Task) -> String {
    if task.environments.is_empty() {
        return "(none)".to_string();
    }

    let mut keys = task.environments.keys().cloned().collect::<Vec<String>>();
    keys.sort();
    keys.join(",")
}

async fn run_tasks(
    config: Config,
    task_names: Option<String>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let parsed_task_names = parse_task_names(task_names);
    let should_execute = |task_name: &str| match &parsed_task_names {
        Some(tasks) => tasks.iter().any(|name| name == task_name),
        None => true,
    };
    let tasks = config
        .tasks
        .into_iter()
        .filter(|task| should_execute(&task.name))
        .collect::<Vec<_>>();

    if dry_run {
        println!("dry-run: {} task(s) selected", tasks.len());
        for (idx, task) in tasks.iter().enumerate() {
            let timeout = task
                .timeout_sec
                .map(|sec| format!("{sec}s"))
                .unwrap_or_else(|| "none".to_string());
            let retry = task.retry.unwrap_or(0);
            println!(
                "[{}/{}] {}: host={} script={} working_dir={} sudo={} timeout={} retry={} env_keys={}",
                idx + 1,
                tasks.len(),
                task.name,
                task.host,
                task.script_path,
                task.working_dir,
                task.sudo,
                timeout,
                retry,
                format_env_keys(task)
            );
        }
        return Ok(());
    }

    let ssh_config = read_ssh_config(SshConfigPath::default())?;

    let mut session_manager = SessionManager::new();
    let client = KuoriClient::new(ssh_config);

    for task in tasks {
        let script_path = Path::new(&task.script_path);
        let working_dir = Path::new(&task.working_dir);
        let max_attempts = task.retry.unwrap_or(0) + 1;

        for attempt in 1..=max_attempts {
            let result = client.exec_script(
                &mut session_manager,
                &task.host,
                script_path,
                working_dir,
                &task.environments,
                task.sudo,
                task.timeout_sec,
            );

            match result {
                Ok(()) => break,
                Err(error) if attempt < max_attempts => {
                    eprintln!(
                        "task `{}` failed (attempt {}/{}): {}",
                        task.name, attempt, max_attempts, error
                    );
                }
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!(
                            "task `{}` failed after {} attempt(s)",
                            task.name, max_attempts
                        )
                    });
                }
            }
        }
    }

    Ok(())
}

fn version_string() -> String {
    let semver = option_env!("KUORI_SEMVER").unwrap_or("unknown");
    let sha = option_env!("KUORI_GIT_SHA").unwrap_or("unknown");
    format!("{semver}+{sha}")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    if args.version {
        println!("{}", version_string());
        return Ok(());
    }

    match args.command {
        Some(CliCommand::Update) => {
            update::run()?;
        }
        Some(CliCommand::Validate(validate_args)) => {
            load_config(&validate_args.config)?;
        }
        Some(CliCommand::Run(run_args)) => {
            let config = load_config(&run_args.config)?;
            run_tasks(config, run_args.task_names, run_args.dry_run).await?;
        }
        None => {
            let config_path = args
                .config
                .as_ref()
                .context("`--config` is required (or use `kuori run|validate --config ...`)")?;
            let config = load_config(config_path)?;
            run_tasks(config, args.task_names, args.dry_run).await?;
        }
    }

    Ok(())
}
