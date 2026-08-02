//! `kms_status` helper. Needs Docker for live container fields.

use kms_secp256k1_api_mcp::ops;

fn main() {
    println!("{}", ops::status());
}
