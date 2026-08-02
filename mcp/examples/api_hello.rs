//! GET / hello against a running API (`KMS_API_URL`).

use kms_secp256k1_api_mcp::client;

#[tokio::main]
async fn main() {
    let msg = std::env::args().nth(1);
    println!("{}", client::hello(msg.as_deref()).await);
}
