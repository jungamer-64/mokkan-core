# OpenID Connect — Authorization Code Flow (PKCE) 使い方

このドキュメントは、mokkan-core が提供する簡易 OpenID/OAuth2 スタイルのエンドポイント (認可コードフロー + PKCE, consent, introspect, revoke) の使い方を説明します。

## 対応エンドポイント

- GET  /api/v1/auth/authorize  — 認可エンドポイント（ブラウザ/リダイレクト）
- POST /api/v1/auth/token      — トークン交換エンドポイント（`application/x-www-form-urlencoded` をサポート）
- POST /api/v1/auth/introspect — トークン・イントロスペクション
- POST /api/v1/auth/revoke     — トークン失効（セッション失効）

## 注意

- `token` エンドポイントは RFC 標準に従い `application/x-www-form-urlencoded` を受け取ることを想定しています。本実装では JSON も受け取れるように柔軟に処理していますが、クライアントはフォームエンコードを使うのが推奨です。
- `authorize` エンドポイントはテスト用に `consent=approve` クエリを渡すことで自動承認できます。UI での同意画面が必要な場合、`authorize` は最初に同意プロンプト JSON を返します。

## 1) authorize の使い方

### パラメータ (主なもの)

- `response_type=code` (必須)
- `client_id` (任意 / テスト用値を使用)
- `redirect_uri` (必須に近い: クライアントで受け取れる安全な URI を使用)
- `scope` (`openid profile` など)
- `state` (CSRF 対策に使用)
- `code_challenge` (PKCE 用)
- `code_challenge_method` (`S256` を推奨)
- `consent=approve` (テスト自動化用)

例: ブラウザまたは curl で認可要求

```sh
# 事前に code_verifier / code_challenge を生成しておく (下記を参照)
AUTHZ_URL="http://localhost:8000/api/v1/auth/authorize?response_type=code&client_id=test-client&redirect_uri=http://localhost:8080/cb&scope=openid%20profile&state=xyz&code_challenge=${CODE_CHALLENGE}&code_challenge_method=S256"
curl -v "$AUTHZ_URL"
```

自動テスト時には `consent=approve` を付けることで、即座にコードを取得できます。

## 2) PKCE (S256) の生成例

シェルで簡単に生成する例 (Python を使用):

```sh
# code_verifier (base64url, padding 削除)
CODE_VERIFIER=$(python - <<'PY'
import os,base64
print(base64.urlsafe_b64encode(os.urandom(32)).rstrip(b"=").decode())
PY
)

# code_challenge: SHA256(code_verifier) -> base64url
CODE_CHALLENGE=$(python - <<PY
import hashlib,base64,sys
cv = sys.argv[1].encode()
digest = hashlib.sha256(cv).digest()
print(base64.urlsafe_b64encode(digest).rstrip(b"=").decode())
PY
"$CODE_VERIFIER")

echo "verifier: $CODE_VERIFIER"
echo "challenge: $CODE_CHALLENGE"
```

（上記は一例です。言語/環境に合わせて PKCE の S256 仕様に従って生成してください。）

## 3) トークン交換（token エンドポイント）

Authorization Code を受け取ったら、トークン交換を行います。推奨は `application/x-www-form-urlencoded` での POST です。

```sh
curl -X POST "http://localhost:8000/api/v1/auth/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=authorization_code&code=PASTE_AUTH_CODE_HERE&redirect_uri=http://localhost:8080/cb&code_verifier=${CODE_VERIFIER}&client_id=test-client"
```

成功すればアクセストークン等を含む JSON (AuthTokenDto) が返却されます。

注意: 本サーバは JSON ボディも受け取るように実装されていますが、互換性のためフォームエンコードを使うことを推奨します。

### 例: トークン交換の成功レスポンス

以下はテスト環境で返される `AuthTokenDto` 形式の例です（実運用ではフィールドが追加/変更されることがあります）。

```json
{
  "token": "issued-1",
  "token_type": "bearer",
  "expires_in": 3600,
  "refresh_token": "refresh-abc123",
  "scope": "openid profile"
}
```

### Node.js での PKCE (S256) 生成サンプル

簡単な Node.js スニペット（標準の crypto を使用）:

```js
// Node.js > 14
const crypto = require('crypto');

function base64UrlEncode(buffer) {
  return buffer.toString('base64').replace(/=/g, '').replace(/\+/g, '-').replace(/\//g, '_');
}

function generatePkcePair() {
  const verifier = base64UrlEncode(crypto.randomBytes(32));
  const challenge = base64UrlEncode(crypto.createHash('sha256').update(verifier).digest());
  return { verifier, challenge };
}

const { verifier, challenge } = generatePkcePair();
console.log('verifier:', verifier);
console.log('challenge:', challenge);
```

## 4) introspect / revoke の使い方

イントロスペクション (トークンの有効性確認):

```sh
curl -X POST "http://localhost:8000/api/v1/auth/introspect" \
  -H "Content-Type: application/json" \
  -d '{"token":"PASTE_TOKEN_HERE"}'
```

リボーク（セッション失効）:

```sh
curl -X POST "http://localhost:8000/api/v1/auth/revoke" \
  -H "Content-Type: application/json" \
  -d '{"token":"PASTE_TOKEN_HERE"}'
```

## 5) テスト参照

テストコード（参考実装）:

- `tests/e2e_authorize_flow.rs` — 認可フローの E2E テスト例（PKCE plain と S256 の検証）

----

補足: 本ドキュメントはサンプル/テスト向けの説明です。実運用では `client_id` の登録管理や `redirect_uri` の厳密なホワイトリスト、TLS の使用、同意画面の UI 実装などを適切に行ってください。
