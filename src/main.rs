mod config;
mod ssh;
mod ssh_config;
mod util;

use config::Config;
use ssh::{KuoriClient, SessionManager};
use ssh_config::{read_ssh_config, SshConfigPath};
use std::{
    fs,
    path::{Path, PathBuf},
};
use structopt::StructOpt;

// コマンドライン引数
#[derive(StructOpt, Debug)]
struct CliArgs {
    #[structopt(short, long, parse(from_os_str))]
    config: PathBuf, // --config で設定ファイルを指定
}

fn load_config(config_path: &PathBuf) -> anyhow::Result<Config> {
    let config_str = fs::read_to_string(config_path)?;
    let config: Config = serde_json::from_str(&config_str)?;
    Ok(config)
}

#[tokio::main]
async fn main() {
    let args = CliArgs::from_args();

    let config = load_config(&args.config).unwrap();
    let ssh_config = read_ssh_config(SshConfigPath::default()).unwrap();

    let mut session_manager = SessionManager::new();
    let client = KuoriClient::new(ssh_config);

    for task in config.tasks {
        let script_path = Path::new(&task.script_path);
        let working_dir = Path::new(&task.working_dir);

        let output = client
            .exec_script(
                &mut session_manager,
                &task.host,
                script_path,
                working_dir,
                &task.environments,
                task.sudo,
            )
            .unwrap();

        println!("Task on host {} output:\n{}", task.host, output);
    }
}
