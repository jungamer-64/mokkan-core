GitHub Actionsで実行したワークフローの完了を待ち、そのログを表示するには、以下のスクリプトを使用できます。このスクリプトは、指定したワークフローの実行IDを使って、そのステータスをポーリングし、完了したらログを取得して表示します。

```bash
---
title: "copilot-instructions (machine-friendly)"
description: "GitHub Actionsで実行したワークフローの完了を待ち、そのログを取得・表示するための、AIが解析しやすい形式の説明とスクリプト。"
version: "1.1"
language: "ja"
author: "automation"
tags: ["workflow","github-actions","script","copilot"]
---

## 概要

このファイルは、GitHub Actions のワークフロー実行（run_id）をポーリングし、完了したら実行結果（conclusion）とログを取得して表示するための短いシェルスクリプトを含みます。  
機械的に解析しやすいように、メタデータ・パラメータ定義・使い方・スクリプトを明確に分離しています。

## パラメータ（機械可読）

```yaml
parameters:
  run_id: "<YOUR_WORKFLOW_RUN_ID>"   # 必須: ワークフロー実行の ID
  repo: "jungamer-64/mokkan-core"    # デフォルト: リポジトリ（owner/repo）
```

## 使い方（人間向け）

1. `run_id` に対象のワークフロー実行 ID をセットします。  
2. 必要であれば `repo` を変更します（owner/repo）。  
3. このスクリプトを実行すると、最長 60 回、5 秒間隔で状態をポーリングし、完了時に結論とログを出力します。

## スクリプト

```bash
#!/bin/bash
run_id=<YOUR_WORKFLOW_RUN_ID>
repo=jungamer-64/mokkan-core

for i in $(seq 1 60); do
  status=$(gh run view "$run_id" -R "$repo" --json status --jq .status 2>/dev/null || echo "error")
  echo "poll $i: $status"

  if [ "$status" = "completed" ]; then
    conclusion=$(gh run view "$run_id" -R "$repo" --json conclusion --jq .conclusion)
    echo "✅ Completed with conclusion: $conclusion"
    gh run view "$run_id" -R "$repo" --log
    break
  fi

  sleep 5
done
```

## 出力（機械可読の期待値）

- ポーリング状況: "poll <n>: <status>" 行が 1 回ごとに出力されます。  
- 完了時: "✅ Completed with conclusion: <conclusion>" が出力され、その後ワークフローのログ全体が表示されます。

## 注意事項

- このスクリプトは `gh` CLI（GitHub CLI）に依存します。事前に `gh auth login` 等で認証済みである必要があります。  
- `run_id` は整数の実行 ID を指定してください。`repo` は owner/repo 形式です。  
- このファイルはドキュメント目的のもので、必要に応じて CI や自動化に組み込んで使ってください。

```

cargoを実行する際は-j4オプションを付与して4スレッドでビルドを行います。