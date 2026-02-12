use anyhow::{bail, Context};
use std::{
    fs,
    fs::File,
    io,
    path::{Path, PathBuf},
    process,
};

const REPOSITORY: &str = "wim-web/kuori";
const LINUX_X86_64_ASSET_NAME: &str = "kuori-x86_64-unknown-linux-gnu";

pub fn run() -> anyhow::Result<()> {
    let asset_name = release_asset_name()?;
    let current_exe =
        std::env::current_exe().context("現在の実行ファイルパスの取得に失敗しました")?;
    let staging_path = staging_binary_path(&current_exe)?;

    download_latest_release(asset_name, &staging_path)?;
    set_executable_permission(&staging_path)?;

    fs::rename(&staging_path, &current_exe).with_context(|| {
        format!(
            "バイナリの置換に失敗しました: {} -> {}",
            staging_path.display(),
            current_exe.display()
        )
    })?;

    println!("kuori を最新バージョンに更新しました。");
    Ok(())
}

fn release_asset_name() -> anyhow::Result<&'static str> {
    if cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        return Ok(LINUX_X86_64_ASSET_NAME);
    }

    bail!("この環境向けの更新は未対応です（linux/x86_64/gnu のみ対応）");
}

fn staging_binary_path(current_exe: &Path) -> anyhow::Result<PathBuf> {
    let parent = current_exe
        .parent()
        .context("実行ファイルディレクトリを特定できませんでした")?;
    Ok(parent.join(format!(".kuori-update-{}", process::id())))
}

fn download_latest_release(asset_name: &str, output_path: &Path) -> anyhow::Result<()> {
    let url = format!(
        "https://github.com/{}/releases/latest/download/{}",
        REPOSITORY, asset_name
    );
    let response = ureq::get(&url)
        .set("User-Agent", "kuori-self-update")
        .call()
        .map_err(|error| anyhow::anyhow!("最新リリースの取得に失敗しました: {}", error))?;
    let status_code = response.status();

    let mut file = File::create(output_path).with_context(|| {
        format!(
            "更新用ファイルの作成に失敗しました: {}",
            output_path.display()
        )
    })?;
    let mut reader = response.into_reader();
    io::copy(&mut reader, &mut file).context("更新バイナリの保存に失敗しました")?;

    if !(200..300).contains(&status_code) {
        bail!("最新リリースの取得に失敗しました: HTTP {}", status_code);
    }

    Ok(())
}

fn set_executable_permission(path: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)?;
    }

    Ok(())
}
