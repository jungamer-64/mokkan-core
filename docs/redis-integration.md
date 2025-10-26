# Redis integration test

This repository contains an optional integration test that validates the Redis-backed session
store's atomic compare-and-swap behavior used during refresh-token rotation.

Test file: `tests/e2e_refresh_rotation_redis.rs`

How to run locally

1. Start a local Redis instance (Docker):

```bash
docker run --name mokkan-redis -p 6379:6379 -d redis:7
```

2. Run the test (the test will read the `REDIS_URL` env var and fall back to `redis://127.0.0.1:6379`):

```bash
# Optional: export REDIS_URL if your Redis is not on localhost
export REDIS_URL="redis://127.0.0.1:6379"
cargo test --test e2e_refresh_rotation_redis
```

Notes

- The test performs a quick PING to the configured Redis endpoint and will skip itself when
  Redis is not available. This keeps CI and local runs flexible.

Configuration

- `REDIS_URL`: connection URL used by the tests (default: `redis://127.0.0.1:6379`).
- `REDIS_USED_NONCE_TTL_SECS`: TTL (in seconds) for the "used refresh nonce" markers created when
  refresh tokens are rotated. Defaults to 604800 (7 days). You can set this to a smaller value for
  local testing or a longer retention period in production.
- `REDIS_PRELOAD_CAS_SCRIPT`: when set to `1` or `true`, the application will attempt to `SCRIPT LOAD`
  the CAS Lua script at startup to avoid first-request latency caused by `SCRIPT LOAD` on demand.
  Note: preloading runs in a background task and requires a Tokio runtime (the store constructor will
  still succeed if preloading fails).
