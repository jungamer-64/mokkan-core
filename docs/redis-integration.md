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
