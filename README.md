# rust_cms_nw

軽量な Rust 製 CMS の雛形リポジトリです。ここにはローカルセットアップ手順、環境変数の説明、マイグレーション方法を記載します。

セットアップ手順 (Linux / macOS / WSL)

1. Rust と Cargo をインストールしてください（<https://rustup.rs>）。
2. リポジトリをクローンして移動します。
3. 環境変数を設定します。サンプルを基に `.env` を作成します:

   cp .env.example .env

   必要に応じて `.env` を編集してください。デフォルトは PostgreSQL を使用します。

4. データベースを作成・マイグレーションを実行します。
   - 例: `createdb cms` などで PostgreSQL データベースを用意します。
   - `DATABASE_URL=postgres://postgres:postgres@localhost:5432/cms sqlx migrate run`

5. ビルドして実行します:

    - 開発モード（デバッグビルド、ファイル変更を反映しやすい）:

       cargo run

    - 本番向け（最適化ビルド）:

       cargo build --release
       cargo run --release

6. テストがあれば次のコマンドで実行できます:

   cargo test

追加情報

- 記事には公開日時 (`published_at`) が追加され、公開時に自動で記録されます。公開状態を解除すると `null` になり、API レスポンスにも反映されます。既存の公開記事についてはマイグレーションで作成日時が公開日時として補完されます。
- 記事一覧 API はカーソル型ページング (`?limit=20&cursor=...`) に移行し、レスポンスには `next_cursor` と `has_more` を含みます。取得済みのカーソルをそのまま次リクエストに指定してください。
- `/api/v1/articles/:id/revisions` エンドポイントで記事のリビジョン履歴を取得できます。更新権限を持つユーザーのみアクセス可能です。

- 環境変数:

   - `BISCUIT_ROOT_PRIVATE_KEY`: Biscuit トークンのルート秘密鍵 (base64/hex)。必要に応じて設定してください。
   - `DATABASE_URL`: データベース接続文字列 (例: postgres://postgres:postgres@localhost:5432/cms)
   - `TOKEN_TTL_SECONDS`: トークンの有効期限 (秒)

問題が発生したら、エラーメッセージを共有してください。ビルドや実行エラーの調査を手伝います。
