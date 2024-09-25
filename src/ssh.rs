use anyhow::{anyhow, Ok};
use ssh2::Session;
use ssh2_config::SshConfig;
use std::{
    collections::{hash_map::Entry, HashMap},
    fs::File,
    io::{self, Read, Write},
    net::TcpStream,
    path::Path,
};

use crate::util::generate_random_string;

pub struct KuoriClient {
    config: SshConfig,
}

pub struct SessionManager {
    sessions: HashMap<String, Session>,
}

impl SessionManager {
    pub fn new() -> Self {
        let sessions = HashMap::new();
        Self { sessions }
    }
}

impl KuoriClient {
    pub fn new(config: SshConfig) -> Self {
        Self { config }
    }

    fn get_session<'a>(
        &self,
        host_name: impl AsRef<str>,
        session_manager: &'a mut SessionManager,
    ) -> anyhow::Result<&'a Session> {
        let session = match session_manager
            .sessions
            .entry(host_name.as_ref().to_string())
        {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(self.connect(host_name)?),
        };

        Ok(session)
    }

    fn connect(&self, host_name: impl AsRef<str>) -> anyhow::Result<Session> {
        let params = self.config.query(host_name);

        let host = params.host_name.ok_or(anyhow!("not found hostname"))?;
        let port = params.port.unwrap_or(22);
        let username = params.user.ok_or(anyhow!("not found user"))?;
        let privatekey_path = params
            .identity_file
            .ok_or(anyhow!("not found identity file"))?;

        // TCP 接続を確立
        let tcp = TcpStream::connect(format!("{}:{}", host, port))?;

        // SSH セッションの作成
        let mut session = Session::new().unwrap();
        session.set_tcp_stream(tcp);
        session.handshake()?;

        // 公開鍵認証を使って接続
        session.userauth_pubkey_file(&username, None, &privatekey_path[0], None)?;

        // 認証が成功したか確認
        if !session.authenticated() {
            anyhow::bail!("SSH 認証に失敗しました");
        }

        Ok(session)
    }

    pub fn exec_script(
        &self,
        session_manager: &mut SessionManager,
        host_name: impl AsRef<str>,
        local_script_path: &Path,
        remote_script_dir: &Path,
        environments: &HashMap<String, String>,
        use_sudo: bool,
    ) -> anyhow::Result<()> {
        let env_command: String = environments
            .iter()
            .map(|(key, value)| format!("export {}={};", key, value))
            .collect::<Vec<String>>()
            .join(" ");

        // セッションを取得
        let session = self.get_session(host_name, session_manager)?;
        let path = remote_script_dir.join(generate_random_string(10));
        let remote_script_path = path
            .to_str()
            .ok_or(anyhow::anyhow!("cannot gen remote_script_path"))?;

        // ローカルスクリプトをリモートに転送
        self.send_script(session, local_script_path, remote_script_path)?;

        let sudo_prefix = if use_sudo { "sudo " } else { "" };
        let command = format!(
            "cd {}; {} {}bash {}",
            remote_script_dir.display(),
            env_command,
            sudo_prefix,
            remote_script_path,
        );

        // リモートでスクリプトを実行
        let result = self.run_remote_command(session, command);

        // スクリプトを削除
        self.run_remote_command(session, format!("rm {}", remote_script_path))?;

        result
    }

    // ローカルスクリプトをリモートに転送
    fn send_script(
        &self,
        session: &Session,
        local_script_path: &Path,
        remote_script_path: &str,
    ) -> anyhow::Result<()> {
        let mut local_file = File::open(local_script_path)?;
        let metadata = local_file.metadata()?;
        let mut remote_file =
            session.scp_send(Path::new(remote_script_path), 0o755, metadata.len(), None)?;

        let mut buffer = Vec::new();
        local_file.read_to_end(&mut buffer)?;
        remote_file.write_all(&buffer)?;

        Ok(())
    }

    // リモートでコマンドを実行して結果を返す
    fn run_remote_command(&self, session: &Session, command: String) -> anyhow::Result<()> {
        let mut channel = session.channel_session()?;
        channel.exec(&command)?;

        let mut buffer = [0; 4096]; // 一度に読み取るバッファサイズを指定
        loop {
            let n = channel.read(&mut buffer)?;
            if n == 0 {
                break; // 読み込みが終了したらループを抜ける
            }
            io::stdout().write_all(&buffer[..n])?; // 取得したデータを即座に標準出力に表示
            io::stdout().flush()?; // バッファをフラッシュしてリアルタイムで表示
        }

        channel.wait_close()?;
        let exit_status = channel.exit_status()?;
        if exit_status != 0 {
            anyhow::bail!("コマンド実行中にエラーが発生しました: {}", exit_status);
        }

        Ok(())
    }
}
