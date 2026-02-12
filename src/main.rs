mod config;
mod ssh;
mod ssh_config;
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
struct CliArgs {
    #[command(subcommand)]
    command: Option<CliCommand>,

    // 後方互換: `kuori --config ...` は `run` として扱う
    #[arg(short, long)]
    config: Option<PathBuf>, // --config で設定ファイルを指定
    #[arg(long)]
    task_names: Option<String>,
}

#[derive(Subcommand, Debug)]
enum CliCommand {
    Run(RunArgs),
    Validate(ValidateArgs),
}

#[derive(Args, Debug)]
struct RunArgs {
    #[arg(short, long)]
    config: PathBuf,
    #[arg(long)]
    task_names: Option<String>,
}

#[derive(Args, Debug)]
struct ValidateArgs {
    #[arg(short, long)]
    config: PathBuf,
}

fn load_config(config_path: &PathBuf) -> anyhow::Result<Config> {
    let config_str = fs::read_to_string(config_path)
        .with_context(|| format!("failed to read config file: {}", config_path.display()))?;
    let config: Config = serde_json::from_str(&config_str)
        .with_context(|| format!("failed to parse config file: {}", config_path.display()))?;
    config
        .validate()
        .with_context(|| format!("invalid config file: {}", config_path.display()))?;
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

async fn run_tasks(config: Config, task_names: Option<String>) -> anyhow::Result<()> {
    let ssh_config = read_ssh_config(SshConfigPath::default())?;

    let mut session_manager = SessionManager::new();
    let client = KuoriClient::new(ssh_config);
    let parsed_task_names = parse_task_names(task_names);

    let should_execute = |task_name: &str| match &parsed_task_names {
        Some(tasks) => tasks.iter().any(|name| name == task_name),
        None => true,
    };

    for task in config.tasks {
        if !should_execute(&task.name) {
            continue;
        }

        let script_path = Path::new(&task.script_path);
        let working_dir = Path::new(&task.working_dir);

        client.exec_script(
            &mut session_manager,
            &task.host,
            script_path,
            working_dir,
            &task.environments,
            task.sudo,
        )?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    match args.command {
        Some(CliCommand::Validate(validate_args)) => {
            load_config(&validate_args.config)?;
        }
        Some(CliCommand::Run(run_args)) => {
            let config = load_config(&run_args.config)?;
            run_tasks(config, run_args.task_names).await?;
        }
        None => {
            let config_path = args
                .config
                .as_ref()
                .context("`--config` is required (or use `kuori run|validate --config ...`)")?;
            let config = load_config(config_path)?;
            run_tasks(config, args.task_names).await?;
        }
    }

    Ok(())
}
