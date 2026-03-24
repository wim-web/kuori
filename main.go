package main

import (
	"fmt"
	"os"
	"strings"

	"github.com/spf13/cobra"
)

var (
	semver = "unknown"
	gitSHA = "unknown"
)

func versionString() string {
	return fmt.Sprintf("%s+%s", semver, gitSHA)
}

func parseTaskNames(taskNames string) []string {
	if taskNames == "" {
		return nil
	}
	var result []string
	for _, name := range strings.Split(taskNames, ",") {
		name = strings.TrimSpace(name)
		if name != "" {
			result = append(result, name)
		}
	}
	return result
}

func runTasks(config *Config, taskNames string, dryRun bool) error {
	parsedNames := parseTaskNames(taskNames)
	shouldExecute := func(name string) bool {
		if parsedNames == nil {
			return true
		}
		for _, n := range parsedNames {
			if n == name {
				return true
			}
		}
		return false
	}

	var tasks []Task
	for _, task := range config.Tasks {
		if shouldExecute(task.Name) {
			tasks = append(tasks, task)
		}
	}

	if dryRun {
		fmt.Printf("dry-run: %d task(s) selected\n", len(tasks))
		for idx, task := range tasks {
			timeout := "none"
			if task.TimeoutSec != nil {
				timeout = fmt.Sprintf("%ds", *task.TimeoutSec)
			}
			retry := uint32(0)
			if task.Retry != nil {
				retry = *task.Retry
			}
			fmt.Printf("[%d/%d] %s: host=%s script=%s working_dir=%s sudo=%v timeout=%s retry=%d env_keys=%s\n",
				idx+1, len(tasks), task.Name, task.Host, task.ScriptPath,
				task.WorkingDir, task.Sudo, timeout, retry, formatEnvKeys(&task))
		}
		return nil
	}

	sm := NewSessionManager()
	defer sm.Close()

	for _, task := range tasks {
		maxAttempts := uint32(1)
		if task.Retry != nil {
			maxAttempts += *task.Retry
		}

		for attempt := uint32(1); attempt <= maxAttempts; attempt++ {
			err := execScript(sm, task.Host, task.ScriptPath, task.WorkingDir,
				task.Environments, task.Sudo, task.TimeoutSec)
			if err == nil {
				break
			}

			if attempt < maxAttempts {
				fmt.Fprintf(os.Stderr, "task `%s` failed (attempt %d/%d): %v\n",
					task.Name, attempt, maxAttempts, err)
			} else {
				return fmt.Errorf("task `%s` failed after %d attempt(s): %w",
					task.Name, maxAttempts, err)
			}
		}
	}

	return nil
}

func main() {
	var showVersion bool
	var rootConfig string
	var rootTaskNames string
	var rootDryRun bool

	rootCmd := &cobra.Command{
		Use:   "kuori",
		Short: "SSH経由で複数ホストへスクリプトを配布・実行するCLIタスクランナー",
		Long: `kuori は JSON で定義したタスクを順番に実行する CLI です。
各タスクは host と script_path を持ち、指定したホストへスクリプトを転送して実行します。
run 実行時には設定ファイルのバリデーションも自動で行われます。`,
		Example: `  kuori validate --config config.json
  kuori run --config config.json
  kuori run --config config.json --task-names deploy-api,restart-worker
  kuori --config config.json  # 後方互換: run として実行`,
		SilenceUsage:  true,
		SilenceErrors: true,
		RunE: func(cmd *cobra.Command, args []string) error {
			if showVersion {
				fmt.Println(versionString())
				return nil
			}

			if rootConfig == "" {
				return fmt.Errorf("`--config` is required (or use `kuori run|validate --config ...`)")
			}

			config, err := loadConfig(rootConfig)
			if err != nil {
				return err
			}
			return runTasks(config, rootTaskNames, rootDryRun)
		},
	}

	rootCmd.Flags().BoolVarP(&showVersion, "version", "v", false, "バージョン情報を表示して終了")
	rootCmd.Flags().StringVarP(&rootConfig, "config", "c", "", "設定ファイル(JSON)のパス（サブコマンド未指定時のみ有効）")
	rootCmd.Flags().StringVar(&rootTaskNames, "task-names", "", "実行対象 task.name をカンマ区切りで指定（サブコマンド未指定時のみ有効）")
	rootCmd.Flags().BoolVar(&rootDryRun, "dry-run", false, "実行せずにタスク一覧を表示（サブコマンド未指定時のみ有効）")

	var runConfig string
	var runTaskNames string
	var runDryRun bool

	runCmd := &cobra.Command{
		Use:   "run",
		Short: "設定ファイルに定義されたタスクを実行",
		Long: `設定ファイルを読み込み、バリデーションに成功したタスクを順番に実行します。
--task-names を指定すると対象タスクを絞り込めます。`,
		SilenceUsage:  true,
		SilenceErrors: true,
		RunE: func(cmd *cobra.Command, args []string) error {
			config, err := loadConfig(runConfig)
			if err != nil {
				return err
			}
			return runTasks(config, runTaskNames, runDryRun)
		},
	}
	runCmd.Flags().StringVarP(&runConfig, "config", "c", "", "設定ファイル(JSON)のパス")
	runCmd.MarkFlagRequired("config")
	runCmd.Flags().StringVar(&runTaskNames, "task-names", "", "実行対象 task.name をカンマ区切りで指定")
	runCmd.Flags().BoolVar(&runDryRun, "dry-run", false, "実行せずにタスク一覧を表示")

	var validateConfig string

	validateCmd := &cobra.Command{
		Use:   "validate",
		Short: "設定ファイルの妥当性のみ検証",
		Long: `設定ファイル(JSON)の形式と必須項目を検証します。
実行は行わず、問題がある場合のみエラーで終了します。`,
		SilenceUsage:  true,
		SilenceErrors: true,
		RunE: func(cmd *cobra.Command, args []string) error {
			_, err := loadConfig(validateConfig)
			return err
		},
	}
	validateCmd.Flags().StringVarP(&validateConfig, "config", "c", "", "検証対象の設定ファイル(JSON)のパス")
	validateCmd.MarkFlagRequired("config")

	updateCmd := &cobra.Command{
		Use:   "update",
		Short: "kuori を最新バージョンへ更新",
		Long:  "GitHub Releases の最新バイナリを使って kuori を自己更新します。",
		SilenceUsage:  true,
		SilenceErrors: true,
		RunE: func(cmd *cobra.Command, args []string) error {
			return runUpdate()
		},
	}

	rootCmd.AddCommand(runCmd, validateCmd, updateCmd)

	if err := rootCmd.Execute(); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}
