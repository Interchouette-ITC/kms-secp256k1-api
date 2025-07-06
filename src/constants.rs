pub static WASM_PATH: &str = "./wasm/wasm.wasm";
pub static CASPER_SECP_PREFIX: &str = "02";
pub static DEFAULT_PORT: u16 = 4000;
pub static DEFAULT_ETH_CHAIN_ID: u8 = 1;
pub const SIGNATURE_RS_LEN: usize = 128;
pub const SIGNATURE_RSV_LEN: usize = 130;

pub static TRANSACTION_HASH: &str =
    "bf2902fc693c1f64978e30557e04844ae74a64f9e07b72bd40a10d46508ed9fb";

pub const CASPER_PUBLIC_KEY_PREFIXED: &str = concat!(
    "02",
    "03f7b225df97085d56397508a6591b961fd21867cddca1210d74379a2b73230886"
);

#[cfg(test)]
pub static SIGNATURE: &str = "3da60aec68fe4c696f8eb86e2aa5a028343adecdcd4db22c769eca91a0bb7a8b0ec804bed0fbc58833fe983cf1cf1354c80bfee10ac25bee91d8d8e3ee51cb5e";

#[cfg(test)]
pub const SIGNATURE_PREFIXED: &str = concat!(
    "02",
    "3da60aec68fe4c696f8eb86e2aa5a028343adecdcd4db22c769eca91a0bb7a8b0ec804bed0fbc58833fe983cf1cf1354c80bfee10ac25bee91d8d8e3ee51cb5e"
);

#[cfg(test)]
pub static SIGNATURE_BASE64: &str = "MEQCID2mCuxo/kxpb464biqloCg0Ot7NzU2yLHaeypGgu3qLAiAOyAS+0PvFiDP+mDzxzxNUyAv+4QrCW+6R2Njj7lHLXg==";

pub static ETH_TRANSACTION_HASH: &str =
    "d2ab5d10a332cdf3222b7ffecb5abd07b44f338be7193775465e10b3e4fe0299";

#[cfg(test)]
pub static ETH_PUBLIC_KEY: &str =
    "02cdf5f74f32842442a6e1263aee727b2bd2accb5ce71525c5ea986501536eac00";

#[cfg(test)]
pub static ETH_SIGNATURE: &str = "ccb866b7e9ed5028d2e2456e734671549c47e560580735f1ff3a29cd2060f481396bbb8fffa855927d4a9179315826e31863091243232cebda1b894d678dce1425";

#[cfg(test)]
pub static ETH_SIGNATURE_V: &str = "25";

// Signature without ETH_SIGNATURE_V
#[cfg(test)]
pub static ETH_SIGNATURE_BASE64: &str = "MEUCIQDMuGa36e1QKNLiRW5zRnFUnEflYFgHNfH/OinNIGD0gQIgOWu7j/+oVZJ9SpF5MVgm4xhjCRJDIyzr2huJTWeNzhQ=";

// Signature with ETH_SIGNATURE_V
// #[cfg(test)]
// pub static ETH_SIGNATURE_BASE64_WITH_V: &str = "MEUCIQDMuGa36e1QKNLiRW5zRnFUnEflYFgHNfH/OinNIGD0gQIgOWu7j/+oVZJ9SpF5MVgm4xhjCRJDIyzr2huJTWeNzhQl";

pub static ETH_TRANSACTION: &str = r#"
{
    "from": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "to": "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    "value": "0x2386f26fc10000",
    "gas": "0x5208",
    "gasPrice": "0x3b9aca00",
    "nonce": "0x0",
    "chainId": "1",
    "data": "0x"
}
"#;
