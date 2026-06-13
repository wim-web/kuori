---
name: renovate-automerge
description: このリポジトリの Renovate PR を調査し、repo固有ルールに従ってマージし、必要に応じてリリースする時に使う。
---

# Renovate Automerge

この skill は、`wim-web/kuori` で Renovate が作成した open PR を確認し、以下のルールに従ってマージ、リリース、または報告する。

## 対象PR

- 作成者が Renovate の open PR のみを対象にする。
- このリポジトリで観測した Renovate PR author: `app/renovate`, `renovate[bot]`
- このリポジトリで観測した Renovate commit author: `renovate[bot]`
- issue comment の author が `renovate` になる場合があるが、PR author が `app/renovate` または `renovate[bot]` でない PR は対象外。
- base branch は `main` のみを対象にする。
- Dependabot や人間が作成した PR は対象外。

## 必ず確認すること

- PR title/body
- changed files
- update type
- Renovate がPR本文に載せた release notes / changelog / compatibility notes
- upstream changelog / release notes / migration guide
- 破壊的変更、deprecated API、設定変更、peer dependency変更、runtime要件変更の有無
- 影響範囲: runtime dependency / dev dependency / build tool / CI / Docker / infra / deploy / database
- check status
- merge conflict の有無
- requested changes / 未解決の人間 review comment の有無
- Renovate の artifact update notice がある場合、その追加変更の内容

## リポジトリ情報

- リポジトリ: `wim-web/kuori`
- default branch: `main`
- package manager / manifest / lockfile:
  - Go module: `go.mod`, `go.sum`
  - aqua registry: `aqua.yaml`
- Renovate 設定: `renovate.json`
  - `local>wim-web/renovate-config` を継承している。
  - Renovate PR 本文では automerge disabled と観測しているため、この skill の判断で手動マージする。
- CI:
  - `.github/workflows/test.yaml`
  - `.github/workflows/install_test.yaml`
  - `.github/workflows/release.yaml`
  - `.github/actions/install/action.yml`
- Docker / Terraform / infra / migrations / db はこのリポジトリでは観測していない。
- `main` は GitHub branch protection なしと観測している。ただしこの skill では下記の check を必須扱いにする。

## マージしてよいもの

以下をすべて満たす PR だけマージしてよい。

- PR author が `app/renovate` または `renovate[bot]`。
- base branch が `main`。
- draft ではない。
- mergeable で、merge conflict がない。
- `test` workflow の check run `test` が success。
- `install test` workflow の check run `release-linux` が success。
- failed / cancelled / timed out / pending / missing の check がない。
- requested changes がなく、人間の未解決コメントがない。
- PR 本文または upstream の release notes / changelog / migration guide を確認でき、破壊的変更や移行作業が不要だと判断できる。
- 変更ファイルが下記の許可ファイルだけ。

許可ファイル:

- `go.mod`
- `go.sum`
- `aqua.yaml`
- `.github/workflows/*.yaml`
- `.github/workflows/*.yml`

許可する Go dependency update:

- `go.mod` と `go.sum` だけを変更する minor / patch update。
- direct dependency と indirect dependency のどちらも対象にしてよい。
- Renovate の artifact update notice で追加の Go module 更新が示されている場合も、`go.mod` / `go.sum` だけの変更で、追加更新が同じ minor / patch 範囲に収まり、release notes で破壊的変更がないと確認できる場合だけマージしてよい。
- `go` directive または toolchain / runtime 要件を上げる変更はマージしてはいけない。

許可する aqua update:

- `aqua.yaml` の `aquaproj/aqua-registry` ref だけを変更する minor / patch update。
- registry の release notes を確認し、このリポジトリで使用するツール定義や aqua 設定形式への破壊的影響がない場合だけマージしてよい。
- `registries` 以外の aqua 設定、checksum 設定、package 定義を追加・変更する PR はマージしてはいけない。

許可する GitHub Actions update:

- `.github/workflows/*.yaml` または `.github/workflows/*.yml` 内の GitHub Action digest update。
- 同一 major version の tag comment に対応する patch / minor update。
- Renovate PR 本文の changelog / compare link を確認し、workflow の権限、secret 使用、artifact、release 作成、実行条件の意味が変わらない場合だけマージしてよい。
- `.github/actions/install/action.yml` を変更する PR は、インストール動作や secret / SSH / release download に影響するためマージしてはいけない。

## マージしてはいけないもの

- major update。
- PR author が `app/renovate` または `renovate[bot]` ではない PR。
- base branch が `main` ではない PR。
- draft PR。
- merge conflict がある PR。
- failed / cancelled / timed out / pending / missing の check がある PR。
- `test` または `release-linux` が success ではない PR。
- requested changes または未解決の人間 review comment がある PR。
- changelog / release notes / migration guide を確認できず、影響範囲を判断できない PR。
- breaking changes、deprecated API、peer dependency変更、runtime要件変更、設定変更の可能性が残る PR。
- Go の `go` directive、toolchain、runtime 要件を上げる PR。
- CLI の runtime dependency 変更で、major update、repo が使っている API の破壊的変更、認証・設定・ファイル形式・通信互換性に関わる明示的な挙動変更が release notes / changelog / compare で確認できる PR。
- `github.com/spf13/cobra`、`github.com/kevinburke/ssh_config`、`golang.org/x/crypto` など CLI の主要 runtime dependency でも、minor / patch update で、変更ファイルが `go.mod` / `go.sum` だけ、必須 check が成功し、repo 内の利用箇所に影響する breaking change や migration が確認されない場合はマージしてよい。
- `.github/actions/install/action.yml` を変更する PR。
- `.github/workflows/release.yaml` の release 作成、permissions、artifact 名、target platform、secret / token 使用、tag trigger の意味を変える PR。
- Docker image、Terraform、infra、deploy、database、migration に関わる PR。
- source code、test code、README、設定例、migration を変更する PR。
- `renovate.json` または Renovate 設定を変更する PR。
- 複数 manager にまたがり、影響範囲が Go / aqua / GitHub Actions の単一カテゴリとして判断できない PR。
- security update であっても、runtime / CI / release / install 動作への影響判断が必要な PR。
- この skill に明記されていない条件の PR。

## 必須 check

GitHub branch protection はないと観測しているが、マージ前に以下を必須 check として扱う。

- workflow `test` の check run `test`: success
- workflow `install test` の check run `release-linux`: success

check が pending / queued / in_progress の場合は待つ。check が存在しない、古い commit の結果しかない、または最新 commit に対する結果か判断できない場合はマージしてはいけない。

## マージ方法

- `gh pr merge PR番号 --squash --delete-branch` を使う。
- GitHub 側で squash merge が使えない場合はマージせず、理由を報告する。
- merge commit や rebase merge は使わない。
- マージ前に PR の head SHA を再確認し、確認した check が最新 head SHA に対応していることを確認する。

## リリース

1件以上の Renovate PR をマージし、今回マージした変更に CLI 配布物へ影響する変更が含まれる場合だけ、リリースも行う。

リリース対象とみなす変更:

- `go.mod` / `go.sum` の変更で、CLI binary に取り込まれる runtime dependency が更新された場合。
- `.github/workflows/release.yaml` の変更で、release 作成、permissions、artifact 名、target platform、secret / token 使用、tag trigger、build command、埋め込まれる version / commit 情報、または GitHub Release に添付される成果物の内容が変わる場合。
- `.github/actions/install/action.yml` はこの skill ではマージ不可だが、別途人間が確認してマージした場合は CLI 配布・更新経路に影響する変更として扱う。

リリース対象とみなさない変更:

- `aqua.yaml` の `aquaproj/aqua-registry` ref だけの更新。
- `.github/workflows/*.yaml` / `.github/workflows/*.yml` の GitHub Action digest 更新で、CI や artifact upload / download の内部実装だけが変わり、release 作成、artifact 名、target platform、成果物内容、tag trigger、secret / token 使用の意味が変わらないもの。
- dev tool、CI helper、lint / test 実行環境だけに閉じ、CLI binary や GitHub Release の成果物内容に影響しない更新。

今回マージしなかった Renovate PR が残っていても、人間確認が必要として明示的に保留した PR は、それだけではリリースを妨げない。

リリース前に必ず確認すること:

- `main` に未 push / 未 commit のローカル変更がない。
- `git fetch origin --tags` で remote tag を最新化している。
- `origin/main` がマージした PR を含む最新状態になっている。
- `go test ./...` が `origin/main` 相当の内容で成功する。
- `v*.*.*` 形式の最新 semver tag を確認している。
- 同じ tag が remote に存在しない。

リリース手順:

- 最新の stable semver tag から patch version を 1 つ上げる。例: 最新が `v2.0.0` なら次は `v2.0.1`。
- pre-release tag、`test`、`test-v1` など `vX.Y.Z` 以外の tag は version 計算に使わない。
- `origin/main` の commit に対して `git tag vX.Y.Z origin/main` を作成する。
- `git push origin vX.Y.Z` で tag を push し、`.github/workflows/release.yaml` の `Release` workflow を起動する。
- `gh run list --workflow Release --branch vX.Y.Z` または該当 tag の run を確認し、release workflow の結果を報告する。
- Release workflow が成功し、GitHub Release に `kuori-linux-amd64` と `kuori-darwin-arm64` が添付されたことを確認する。
- release 完了後、ローカルの CLI も更新する。
  - `command -v kuori` でローカルの `kuori` が見つかることを確認する。
  - `kuori update` を実行して、GitHub Releases の latest binary に自己更新する。
  - `kuori --version` を実行し、出力がリリースした semver と対象 commit の short SHA に対応していることを確認する。

リリースしてはいけない場合:

- 今回この skill でマージした Renovate PR が 0 件。
- 今回この skill でマージした Renovate PR に、CLI 配布物へ影響する変更が 1 件もない。
- 今回マージ可能と判断した Renovate PR の処理が完了していない。
- `origin/main` の最新化、local test、tag 計算、tag 重複確認のいずれかができない。
- `go test ./...` が失敗する。
- 既存 tag から次の patch version を一意に決められない。
- release workflow が失敗した場合は追加修正せず、失敗として報告する。
- ローカルの `kuori` が見つからない、または `kuori update` が失敗した場合は、release は完了扱いにしたうえで CLI 更新失敗として報告する。

## 報告

- マージした PR は、PR 番号、title、更新対象、merge method を報告する。
- リリースした場合は、tag、対象 commit SHA、Release workflow の URL または run ID、結果を報告する。
- ローカル CLI を更新した場合は、`kuori --version` の結果を報告する。
- ローカル CLI を更新できなかった場合は、その理由を報告する。
- マージしなかった PR は、PR 番号、title、対応が必要な理由を報告する。
- マージしなかった対象 Renovate PR には、`gh pr comment` でマージしない判断理由をコメントとして残す。
  - コメントには、どの skill 条件に該当したか、確認した upstream changelog / release notes / compare の要点、影響範囲、次に人間が確認すべき観点を含める。
  - check failure / pending / merge conflict / requested changes / 未解決 review comment が理由の場合も、その状態を簡潔に記録する。
  - 既に同等内容の保留コメントがある場合は、重複コメントを増やさず、その既存コメント URL を報告する。
  - 既存コメントで十分な場合は、その PR は comment skipped として扱い、`renovate-needs-manual-review` を付ける。重複コメントは投稿しない。
- 対象となる Renovate open PR がない場合は「対象なし」と報告する。
- 判断に必要な release notes や migration guide が見つからない場合は、マージせず確認不能として報告する。

## 禁止操作

- Renovate branch に commit や push をしない。
- PR を close しない。
- Renovate の rebase checkbox を操作しない。
- Renovate 設定を変更しない。
- source code や lockfile を手で修正してマージ条件を満たそうとしない。
- release workflow が失敗しても、その場で修正 commit を作らない。
- この skill に明記されていない条件の PR はマージしない。
