//! LocalStack start → status → stop. Needs Docker.

use kms_secp256k1_api_mcp::ops;

fn main() {
    println!("=== kms_docker_run_localstack ===");
    println!("{}", ops::docker_run_localstack());
    println!("\n=== kms_status ===");
    println!("{}", ops::status());
    println!("\n=== kms_docker_stop_localstack ===");
    println!("{}", ops::docker_stop_localstack());
}
