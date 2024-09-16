use ssh2_config::{ParseRule, SshConfig};
use std::{env, fs::File, io::BufReader};

pub struct SshConfigPath(pub String);

impl Default for SshConfigPath {
    fn default() -> Self {
        let home_dir = env::var("HOME").unwrap();
        let config_path = format!("{}/.ssh/config", home_dir);
        Self(config_path)
    }
}

pub fn read_ssh_config(path: SshConfigPath) -> anyhow::Result<SshConfig> {
    let mut reader = BufReader::new(File::open(path.0).expect("Could not open configuration file"));
    let config = SshConfig::default().parse(&mut reader, ParseRule::STRICT)?;

    Ok(config)
}
