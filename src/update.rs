use anyhow::{bail, Context};
use std::{
    fs,
    fs::File,
    io,
    path::{Path, PathBuf},
    process::{self, Command},
};

const REPOSITORY: &str = "wim-web/kuori";
const REPOSITORY_GIT_URL: &str = "https://github.com/wim-web/kuori.git";
const PACKAGE_NAME: &str = "kuori";
const LINUX_X86_64_ASSET_NAME: &str = "kuori-x86_64-unknown-linux-gnu";

struct ReleaseTag {
    tag: String,
    semver: String,
    sha: String,
}

pub fn run() -> anyhow::Result<()> {
    if cfg!(target_os = "macos") {
        return run_update_via_cargo_install();
    }

    if cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        return run_update_via_release_asset();
    }

    bail!("この環境向けの更新は未対応です");
}

fn run_update_via_release_asset() -> anyhow::Result<()> {
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

fn run_update_via_cargo_install() -> anyhow::Result<()> {
    let latest = latest_tag_from_remote()?;
    let short_sha: String = latest.sha.chars().take(7).collect();

    let output = Command::new("cargo")
        .args([
            "install",
            "--git",
            REPOSITORY_GIT_URL,
            "--tag",
            &latest.tag,
            "--force",
            PACKAGE_NAME,
        ])
        .env("KUORI_SEMVER", &latest.semver)
        .env("KUORI_GIT_SHA", &short_sha)
        .output()
        .context("`cargo` コマンドの実行に失敗しました")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("`cargo install` に失敗しました: {}", stderr.trim());
    }

    println!(
        "kuori を最新バージョンに更新しました。tag: {}, version: {}+{}",
        latest.tag, latest.semver, short_sha
    );
    Ok(())
}

fn latest_tag_from_remote() -> anyhow::Result<ReleaseTag> {
    let output = Command::new("git")
        .args(["ls-remote", "--tags", "--refs", REPOSITORY_GIT_URL])
        .output()
        .context("`git` コマンドの実行に失敗しました")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("最新タグ取得に失敗しました: {}", stderr.trim());
    }

    let stdout = String::from_utf8(output.stdout).context("タグ一覧のデコードに失敗しました")?;
    let tag = pick_latest_semver_tag(&stdout).context("semver形式のタグが見つかりませんでした")?;
    let semver = semver_without_prefix(&tag)
        .context("最新タグのsemver解析に失敗しました")?
        .to_string();
    let sha = resolve_tag_sha(&tag)?;

    Ok(ReleaseTag { tag, semver, sha })
}

fn pick_latest_semver_tag(input: &str) -> Option<String> {
    input
        .lines()
        .filter_map(parse_semver_tag_from_ls_remote_line)
        .max_by(|(_, a), (_, b)| a.cmp(b))
        .map(|(tag, _)| tag)
}

fn parse_semver_tag_from_ls_remote_line(line: &str) -> Option<(String, (u64, u64, u64))> {
    let (_, ref_name) = line.split_once('\t')?;
    let tag = ref_name.strip_prefix("refs/tags/")?;
    let version = parse_semver(tag)?;
    Some((tag.to_string(), version))
}

fn parse_semver(tag: &str) -> Option<(u64, u64, u64)> {
    let version = tag.strip_prefix('v')?;
    let mut parts = version.split('.');

    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;

    if parts.next().is_some() {
        return None;
    }

    Some((major, minor, patch))
}

fn semver_without_prefix(tag: &str) -> Option<&str> {
    parse_semver(tag)?;
    tag.strip_prefix('v')
}

fn resolve_tag_sha(tag: &str) -> anyhow::Result<String> {
    let peeled_ref = format!("refs/tags/{tag}^{{}}");
    if let Some(sha) = ls_remote_sha_for_ref(&peeled_ref)? {
        return Ok(sha);
    }

    let direct_ref = format!("refs/tags/{tag}");
    if let Some(sha) = ls_remote_sha_for_ref(&direct_ref)? {
        return Ok(sha);
    }

    bail!("タグ {} のSHAが取得できませんでした", tag);
}

fn ls_remote_sha_for_ref(reference: &str) -> anyhow::Result<Option<String>> {
    let output = Command::new("git")
        .args(["ls-remote", REPOSITORY_GIT_URL, reference])
        .output()
        .context("`git` コマンドの実行に失敗しました")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("タグSHA取得に失敗しました: {}", stderr.trim());
    }

    let stdout = String::from_utf8(output.stdout).context("タグSHAのデコードに失敗しました")?;
    let line = stdout.lines().next();
    Ok(line.and_then(parse_sha_from_ls_remote_line))
}

fn parse_sha_from_ls_remote_line(line: &str) -> Option<String> {
    let (sha, _) = line.split_once('\t')?;
    if sha.is_empty() {
        return None;
    }
    Some(sha.to_string())
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

#[cfg(test)]
mod tests {
    use super::{parse_sha_from_ls_remote_line, pick_latest_semver_tag, semver_without_prefix};

    #[test]
    fn picks_latest_tag() {
        let input = "\
aaaaaaaa\trefs/tags/v1.2.9
bbbbbbbb\trefs/tags/v1.10.0
cccccccc\trefs/tags/v1.3.0
";
        let latest = pick_latest_semver_tag(input);
        assert_eq!(latest.as_deref(), Some("v1.10.0"));
    }

    #[test]
    fn ignores_non_semver_tags() {
        let input = "\
aaaaaaaa\trefs/tags/latest
bbbbbbbb\trefs/tags/v1.2
cccccccc\trefs/tags/v2.0.0-rc.1
dddddddd\trefs/tags/v2.0.0
";
        let latest = pick_latest_semver_tag(input);
        assert_eq!(latest.as_deref(), Some("v2.0.0"));
    }

    #[test]
    fn strips_v_prefix_from_semver_tag() {
        assert_eq!(semver_without_prefix("v1.2.3"), Some("1.2.3"));
        assert_eq!(semver_without_prefix("latest"), None);
    }

    #[test]
    fn parses_sha_from_ls_remote_line() {
        let line = "0123456789abcdef\trefs/tags/v1.0.0";
        let sha = parse_sha_from_ls_remote_line(line);
        assert_eq!(sha.as_deref(), Some("0123456789abcdef"));
    }
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
