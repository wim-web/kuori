use anyhow::anyhow;
use ssh2::Session;
use ssh2_config::SshConfig;
use std::{
    collections::{hash_map::Entry, HashMap},
    fs::File,
    io::{self, Read, Write},
    net::TcpStream,
    path::Path,
    thread,
    time::{Duration, Instant},
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
        timeout_sec: Option<u64>,
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
        let result = self.run_remote_command(session, command, timeout_sec);

        // スクリプトを削除
        self.run_remote_command(session, format!("rm {}", remote_script_path), None)?;

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
    fn run_remote_command(
        &self,
        session: &Session,
        command: String,
        timeout_sec: Option<u64>,
    ) -> anyhow::Result<()> {
        let mut channel = session.channel_session()?;
        channel.exec(&command)?;

        let timeout = timeout_sec.map(Duration::from_secs);
        let started_at = Instant::now();

        session.set_blocking(false);
        let mut stdout_eof = false;
        let mut stderr_eof = false;
        let mut stdout_buffer = [0; 4096];
        let mut stderr_buffer = [0; 4096];

        loop {
            let mut has_output = false;

            if !stdout_eof {
                match channel.read(&mut stdout_buffer) {
                    Ok(0) => stdout_eof = true,
                    Ok(n) => {
                        io::stdout().write_all(&stdout_buffer[..n])?;
                        io::stdout().flush()?;
                        has_output = true;
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                    Err(error) => {
                        session.set_blocking(true);
                        return Err(error.into());
                    }
                }
            }

            if !stderr_eof {
                match channel.stderr().read(&mut stderr_buffer) {
                    Ok(0) => stderr_eof = true,
                    Ok(n) => {
                        io::stderr().write_all(&stderr_buffer[..n])?;
                        io::stderr().flush()?;
                        has_output = true;
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                    Err(error) => {
                        session.set_blocking(true);
                        return Err(error.into());
                    }
                }
            }

            if channel.eof() && stdout_eof && stderr_eof {
                break;
            }

            if let Some(limit) = timeout {
                if started_at.elapsed() > limit {
                    let _ = channel.close();
                    session.set_blocking(true);
                    anyhow::bail!(
                        "コマンド実行がタイムアウトしました ({}秒): {}",
                        limit.as_secs(),
                        command
                    );
                }
            }

            if !has_output {
                thread::sleep(Duration::from_millis(50));
            }
        }

        session.set_blocking(true);
        channel.wait_close()?;
        let exit_status = channel.exit_status()?;
        if exit_status != 0 {
            anyhow::bail!("コマンド実行中にエラーが発生しました: {}", exit_status);
        }

        Ok(())
    }
}
