# rust_cms_nw

軽量な Rust 製 CMS の雛形リポジトリです。ここにはローカルセットアップ手順、環境変数の説明、マイグレーション方法を記載します。

セットアップ手順 (Linux / macOS / WSL)

1. Rust と Cargo をインストールしてください（<https://rustup.rs>）。
2. リポジトリをクローンして移動します。
3. 環境変数を設定します。サンプルを基に `.env` を作成します:

   cp .env.example .env

   必要に応じて `.env` を編集してください。デフォルトは SQLite のファイルベース DB を使用します。

4. データベースを作成・マイグレーションを実行します。
   - このプロジェクトは `migrations/` ディレクトリ内の SQL を使って初期テーブルを作成できます。例えば sqlite の場合:

     mkdir -p data
     sqlite3 ./data/dev.db < migrations/0001_create_tables.sql

5. ビルドして実行します:

    - 開発モード（デバッグビルド、ファイル変更を反映しやすい）:

       cargo run

    - 本番向け（最適化ビルド）:

       cargo build --release
       cargo run --release

6. テストがあれば次のコマンドで実行できます:

   cargo test

追加情報

- 環境変数:

   - `BISCUIT_ROOT_PRIVATE_KEY`: Biscuit トークンのルート秘密鍵 (base64/hex)。必要に応じて設定してください。
   - `DATABASE_URL`: データベース接続文字列 (例: sqlite://./data/dev.db)
   - `TOKEN_TTL_SECONDS`: トークンの有効期限 (秒)

問題が発生したら、エラーメッセージを共有してください。ビルドや実行エラーの調査を手伝います。
