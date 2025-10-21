// src/bin/write_openapi_snapshot.rs
use anyhow::Result;
use std::env;

fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let output_path = env::var("OPENAPI_SNAPSHOT_PATH")
        .unwrap_or_else(|_| "backend/spec/openapi.json".to_string());
    mokkan_core::presentation::http::openapi::write_openapi_snapshot()?;
    println!("OpenAPI snapshot written to {output_path}");
    Ok(())
}
