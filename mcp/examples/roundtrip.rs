//! Host mock API start → createKey → stop. Needs cargo build of API crate.

use kms_secp256k1_api_mcp::client;
use kms_secp256k1_api_mcp::ops;
use kms_secp256k1_api_mcp::paths::DEFAULT_API_URL;
use std::env;
use std::thread;
use std::time::Duration;

#[tokio::main]
async fn main() {
    if env::var("KMS_API_URL").ok().as_deref() != Some(DEFAULT_API_URL) {
        let status = std::process::Command::new(env::current_exe().expect("example executable"))
            .args(env::args_os().skip(1))
            .env("KMS_API_URL", DEFAULT_API_URL)
            .status()
            .expect("run example with API URL");
        std::process::exit(status.code().unwrap_or(1));
    }
    println!("=== kms_api_start ===");
    println!("{}", ops::api_start(Some("casper"), Some(4000)));

    for _ in 0..30 {
        let hello = client::hello(None).await;
        if hello.contains("HTTP 200") || hello.to_lowercase().contains("kms") {
            break;
        }
        thread::sleep(Duration::from_secs(2));
    }

    println!("\n=== kms_hello ===");
    println!("{}", client::hello(Some("mcp-roundtrip")).await);

    println!("\n=== kms_create_key ===");
    println!("{}", client::create_key().await);

    println!("\n=== kms_list_keys ===");
    println!("{}", client::list_keys().await);

    println!("\n=== kms_api_stop ===");
    println!("{}", ops::api_stop());
}
