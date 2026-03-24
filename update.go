package main

import (
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
)

const (
	repository       = "wim-web/kuori"
	repositoryGitURL = "https://github.com/wim-web/kuori.git"
)

type releaseTag struct {
	tag    string
	semver string
	sha    string
}

func releaseAssetName() string {
	return fmt.Sprintf("kuori-%s-%s", runtime.GOOS, runtime.GOARCH)
}

func runUpdate() error {
	assetName := releaseAssetName()
	currentExe, err := os.Executable()
	if err != nil {
		return fmt.Errorf("現在の実行ファイルパスの取得に失敗しました: %w", err)
	}

	stagingPath := filepath.Join(filepath.Dir(currentExe), fmt.Sprintf(".kuori-update-%d", os.Getpid()))

	if err := downloadLatestRelease(assetName, stagingPath); err != nil {
		return err
	}

	if err := os.Chmod(stagingPath, 0o755); err != nil {
		os.Remove(stagingPath)
		return fmt.Errorf("パーミッション設定に失敗しました: %w", err)
	}

	if err := os.Rename(stagingPath, currentExe); err != nil {
		os.Remove(stagingPath)
		return fmt.Errorf("バイナリの置換に失敗しました: %s -> %s: %w", stagingPath, currentExe, err)
	}

	fmt.Println("kuori を最新バージョンに更新しました。")
	return nil
}

func downloadLatestRelease(assetName, outputPath string) error {
	url := fmt.Sprintf("https://github.com/%s/releases/latest/download/%s", repository, assetName)

	resp, err := http.Get(url)
	if err != nil {
		return fmt.Errorf("最新リリースの取得に失敗しました: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("最新リリースの取得に失敗しました: HTTP %d", resp.StatusCode)
	}

	file, err := os.Create(outputPath)
	if err != nil {
		return fmt.Errorf("更新用ファイルの作成に失敗しました: %s: %w", outputPath, err)
	}
	defer file.Close()

	if _, err := io.Copy(file, resp.Body); err != nil {
		return fmt.Errorf("更新バイナリの保存に失敗しました: %w", err)
	}

	return nil
}

func latestTagFromRemote() (*releaseTag, error) {
	output, err := exec.Command("git", "ls-remote", "--tags", "--refs", repositoryGitURL).Output()
	if err != nil {
		return nil, fmt.Errorf("最新タグ取得に失敗しました: %w", err)
	}

	tag := pickLatestSemverTag(string(output))
	if tag == "" {
		return nil, fmt.Errorf("semver形式のタグが見つかりませんでした")
	}

	sv := semverWithoutPrefix(tag)
	if sv == "" {
		return nil, fmt.Errorf("最新タグのsemver解析に失敗しました")
	}

	sha, err := resolveTagSHA(tag)
	if err != nil {
		return nil, err
	}

	return &releaseTag{tag: tag, semver: sv, sha: sha}, nil
}

func pickLatestSemverTag(input string) string {
	var bestTag string
	var bestVersion [3]uint64

	for _, line := range strings.Split(strings.TrimSpace(input), "\n") {
		if tag, version, ok := parseSemverTagFromLsRemoteLine(line); ok {
			if bestTag == "" || compareVersions(version, bestVersion) > 0 {
				bestTag = tag
				bestVersion = version
			}
		}
	}

	return bestTag
}

func parseSemverTagFromLsRemoteLine(line string) (string, [3]uint64, bool) {
	parts := strings.SplitN(line, "\t", 2)
	if len(parts) != 2 {
		return "", [3]uint64{}, false
	}

	refName := parts[1]
	tag := strings.TrimPrefix(refName, "refs/tags/")
	if tag == refName {
		return "", [3]uint64{}, false
	}

	version, ok := parseSemver(tag)
	if !ok {
		return "", [3]uint64{}, false
	}

	return tag, version, true
}

func parseSemver(tag string) ([3]uint64, bool) {
	version := strings.TrimPrefix(tag, "v")
	if version == tag {
		return [3]uint64{}, false
	}

	parts := strings.Split(version, ".")
	if len(parts) != 3 {
		return [3]uint64{}, false
	}

	var result [3]uint64
	for i, part := range parts {
		if strings.ContainsAny(part, "-+") {
			return [3]uint64{}, false
		}
		n, err := strconv.ParseUint(part, 10, 64)
		if err != nil {
			return [3]uint64{}, false
		}
		result[i] = n
	}

	return result, true
}

func compareVersions(a, b [3]uint64) int {
	for i := 0; i < 3; i++ {
		if a[i] > b[i] {
			return 1
		}
		if a[i] < b[i] {
			return -1
		}
	}
	return 0
}

func semverWithoutPrefix(tag string) string {
	if _, ok := parseSemver(tag); !ok {
		return ""
	}
	return strings.TrimPrefix(tag, "v")
}

func resolveTagSHA(tag string) (string, error) {
	peeledRef := fmt.Sprintf("refs/tags/%s^{}", tag)
	if sha, err := lsRemoteSHAForRef(peeledRef); err == nil && sha != "" {
		return sha, nil
	}

	directRef := fmt.Sprintf("refs/tags/%s", tag)
	if sha, err := lsRemoteSHAForRef(directRef); err == nil && sha != "" {
		return sha, nil
	}

	return "", fmt.Errorf("タグ %s のSHAが取得できませんでした", tag)
}

func lsRemoteSHAForRef(reference string) (string, error) {
	output, err := exec.Command("git", "ls-remote", repositoryGitURL, reference).Output()
	if err != nil {
		return "", fmt.Errorf("タグSHA取得に失敗しました: %w", err)
	}

	lines := strings.TrimSpace(string(output))
	if lines == "" {
		return "", nil
	}

	firstLine := strings.SplitN(lines, "\n", 2)[0]
	return parseSHAFromLsRemoteLine(firstLine), nil
}

func parseSHAFromLsRemoteLine(line string) string {
	parts := strings.SplitN(line, "\t", 2)
	if len(parts) != 2 || parts[0] == "" {
		return ""
	}
	return parts[0]
}
