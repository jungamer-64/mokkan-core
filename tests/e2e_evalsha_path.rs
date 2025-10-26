use std::env;
use tokio::time::{sleep, Duration};
use mokkan_core::application::ports::session_revocation::SessionRevocationStore;

mod support;

use mokkan_core::infrastructure::security::redis_session_store::RedisSessionRevocationStore;

#[tokio::test]
async fn script_loads_and_evalsha_path_behavior() {
    let url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());

    // Small delay in case Redis is starting in CI.
    sleep(Duration::from_millis(200)).await;

    // Quick TCP check
    let host_port = {
        let mut s = url.as_str();
        if let Some(i) = s.find("://") { s = &s[i+3..]; }
        if let Some(i) = s.rfind('/') { s = &s[..i]; }
        if let Some(i) = s.rfind('@') { s = &s[i+1..]; }
        s.to_string()
    };

    match tokio::time::timeout(Duration::from_secs(2), tokio::net::TcpStream::connect(host_port)).await {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => {
            eprintln!("Skipping evalsha path test because connection failed: {}", e);
            return;
        }
        Err(_) => {
            eprintln!("Skipping evalsha path test because connection timed out");
            return;
        }
    }

    // Create store without preloading so first use will trigger SCRIPT LOAD.
    let store = RedisSessionRevocationStore::from_url_with_options(&url, 60 * 60 * 24 * 7, false).expect("create store");

    let session_id = format!("evalsha-test-{}", chrono::Utc::now().timestamp());

    // Ensure initial count is zero
    assert_eq!(store.script_loads(), 0);

    // Set initial nonce and perform compare-and-swap -> should load script once.
    store.set_session_refresh_nonce(&session_id, "n1").await.expect("set nonce");
    let ok = store.compare_and_swap_session_refresh_nonce(&session_id, "n1", "n2").await.expect("cas");
    assert!(ok, "first CAS should succeed");
    assert_eq!(store.script_loads(), 1, "script should have been loaded once");

    // Second CAS (rotate n2 -> n3) should use EVALSHA (no additional SCRIPT LOAD)
    let ok2 = store.compare_and_swap_session_refresh_nonce(&session_id, "n2", "n3").await.expect("cas2");
    assert!(ok2, "second CAS should succeed");
    assert_eq!(store.script_loads(), 1, "script should not have been loaded again");

    // Now flush server scripts to force NOSCRIPT on next EVALSHA attempt.
    // Use a blocking redis client in a spawn_blocking to avoid blocking the async runtime.
    let url_clone = url.clone();
    let _ = tokio::task::spawn_blocking(move || {
        if let Ok(client) = redis::Client::open(url_clone) {
            if let Ok(mut conn) = client.get_connection() {
                let _ : redis::RedisResult<()> = redis::cmd("SCRIPT").arg("FLUSH").query(&mut conn);
            }
        }
    }).await;

    // Next CAS should detect NOSCRIPT, reload, and increment the load counter.
    let ok3 = store.compare_and_swap_session_refresh_nonce(&session_id, "n3", "n4").await.expect("cas3");
    assert!(ok3, "third CAS should succeed after reload");
    assert_eq!(store.script_loads(), 2, "script should have been loaded a second time after NOSCRIPT");
}
