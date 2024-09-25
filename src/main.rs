mod config;
mod ssh;
mod ssh_config;
mod util;

use clap::{arg, Parser};
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
    #[arg(short, long)]
    config: PathBuf, // --config で設定ファイルを指定
    #[arg(long)]
    task_names: Option<String>,
}

fn load_config(config_path: &PathBuf) -> anyhow::Result<Config> {
    let config_str = fs::read_to_string(config_path)?;
    let config: Config = serde_json::from_str(&config_str)?;
    Ok(config)
}

#[tokio::main]
async fn main() {
    let args = CliArgs::parse();

    let config = load_config(&args.config).unwrap();
    let ssh_config = read_ssh_config(SshConfigPath::default()).unwrap();

    let mut session_manager = SessionManager::new();
    let client = KuoriClient::new(ssh_config);

    let task_names = args.task_names.map(|names| {
        names
            .split(',')
            .map(|name| name.to_string())
            .collect::<Vec<String>>()
    });

    let should_execute = |task: String| match &task_names {
        Some(tasks) => tasks.contains(&task),
        None => true,
    };

    for task in config.tasks {
        if !should_execute(task.name) {
            continue;
        }

        let script_path = Path::new(&task.script_path);
        let working_dir = Path::new(&task.working_dir);

        client
            .exec_script(
                &mut session_manager,
                &task.host,
                script_path,
                working_dir,
                &task.environments,
                task.sudo,
            )
            .unwrap();
    }
}
