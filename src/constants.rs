pub static WASM_PATH: &str = "./wasm/wasm.wasm";
pub static DEFAULT_APP_PORT: u16 = 4000;

pub static DEFAULT_ETH_CHAIN_ID: u8 = 1;

pub static DEFAULT_COSMOS_CHAIN_ID: &str = "testing";
pub static DEFAULT_COSMOS_HRP: &str = "cosmos";
pub static DEFAULT_COSMOS_REST_URL: &str = "http://localhost:1317/cosmos/auth/v1beta1/accounts/";

pub static CASPER_SECP_PREFIX: &str = "02";
pub static CASPER_SECP_LEN: usize = 68;

pub const SIGNATURE_RS_LEN: usize = 128;
pub const SIGNATURE_RSV_LEN: usize = 130;

#[cfg(test)]
pub static RANDOM_PUBLIC_KEY_BASE64: &str = "MFYwEAYHKoZIzj0CAQYFK4EEAAoDQgAE3J6XOS0V4sy7ae2kXPBmHRiErhGhb8SkCb6ggoJK4hK9+2x1sbKMc7C+KhvluJipttaCpc8Q36q9CFhvAjDcZQ==";

pub static TRANSACTION_HASH: &str =
    "bf2902fc693c1f64978e30557e04844ae74a64f9e07b72bd40a10d46508ed9fb";

pub const CASPER_PUBLIC_KEY_PREFIXED: &str = concat!(
    "02",
    "03ed0688448edc2ed3eb478a52685c6a42dddb2ffa1cd3295a9d338b672a423196"
);

#[cfg(test)]
pub static CASPER_PUBLIC_KEY_BASE64: &str = "MFYwEAYHKoZIzj0CAQYFK4EEAAoDQgAE7QaIRI7cLtPrR4pSaFxqQt3bL/oc0ylanTOLZypCMZYzArcNr2zcQui5G5nWHdCDqLwqIW+eURV8xCFGVgBcPQ==";

#[cfg(test)]
pub static SIGNATURE: &str = "9232039e61f9542971ea2ba7dc4c3e68c9f2dbb9aba58dda17bc0ffe94a6b1b37d485461f6648d737cef62c9df334d7fbdf36054a17f12df23710e4568f2e9fe";

#[cfg(test)]
pub const SIGNATURE_PREFIXED: &str = concat!(
    "02",
    "9232039e61f9542971ea2ba7dc4c3e68c9f2dbb9aba58dda17bc0ffe94a6b1b37d485461f6648d737cef62c9df334d7fbdf36054a17f12df23710e4568f2e9fe"
);

#[cfg(test)]
pub static SIGNATURE_BASE64: &str = "MEUCIQCSMgOeYflUKXHqK6fcTD5oyfLbuauljdoXvA/+lKaxswIgfUhUYfZkjXN872LJ3zNNf73zYFShfxLfI3EORWjy6f4=";

pub static ETH_SECP_LEN: usize = 66;

pub static ETH_TRANSACTION_HASH: &str =
    "d2ab5d10a332cdf3222b7ffecb5abd07b44f338be7193775465e10b3e4fe0299";

#[cfg(test)]
pub static ETH_PUBLIC_KEY: &str =
    "02eb9e5d80cf250496a29c15755afc3a8066241b9b29167cb733e8c7d060e47c33";

#[cfg(test)]
pub static ETH_PUBLIC_KEY_BASE64: &str = "MFYwEAYHKoZIzj0CAQYFK4EEAAoDQgAE655dgM8lBJainBV1Wvw6gGYkG5spFny3M+jH0GDkfDOWE2ghmqonfTHH/wHNPlVX4HQGt4HcAUebrlER8+exQA==";

#[cfg(test)]
pub static ETH_ADDRESS: &str = "0x2fcd6e009e1eb8ebc6ce0d07173965fb0a714dd7";

#[cfg(test)]
pub static ETH_SIGNATURE: &str = "16539fc6382dab4170dafc17bdca64950b8f458a4473a35f6f10a832dfad14953f100297cde512d453220cde6632a356abb89549c7e2461588021795ad8dbaa325";

#[cfg(test)]
pub static ETH_SIGNATURE_V: &str = "25";

#[cfg(test)]
pub static ETH_SIGNATURE_BASE64: &str = "MEUCIBZTn8Y4LatBcNr8F73KZJULj0WKRHOjX28QqDLfrRSVAiEAwO/9aDIa7Sus3fMhmc1cqA72R5znZlomN9BG9yKohp4=";

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

pub static COSMOS_SECP_LEN: usize = 66;

pub static COSMOS_TRANSACTION_HASH: &str =
    "c2b22e8f756bcabe2de238a49c09b2959f5ef340f8b2014a691d8272661e9781";

#[cfg(test)]
pub static COSMOS_PUBLIC_KEY: &str =
    "038a0882db1bdc3c0be2ed124d38773d346c9a2d6b9915cc1155d9bd11f7a08de1";

#[cfg(test)]
pub static COSMOS_PUBLIC_KEY_BASE64: &str = "MFYwEAYHKoZIzj0CAQYFK4EEAAoDQgAEigiC2xvcPAvi7RJNOHc9NGyaLWuZFcwRVdm9EfegjeGuuHXfnVD7KL+eE682JwhlvsgeWay8AEtqPglvNhx0ZQ==";

#[cfg(test)]
pub static COSMOS_ADDRESS: &str = "cosmos1f9ekqvthzry4lypg4zmpfjll72n89wvkatddze";

#[cfg(test)]
pub static COSMOS_SIGNATURE: &str = "d5e46055a8648f1b3e62cbe990db4a3283606f8aa6fa16d6059892d59e9f0f07499950904b429d22ac3901404eb32bf2348ef61d9e974ed81d5a3df52e8a7f66";

#[cfg(test)]
pub static COSMOS_SIGNATURE_BASE64: &str = "MEYCIQDV5GBVqGSPGz5iy+mQ20oyg2Bviqb6FtYFmJLVnp8PBwIhALZmr2+0vWLdU8b+v7FM1AyGH+bJELFRY6J4IJehq8Hb";

pub static COSMOS_TRANSACTION: &str = r#"
{
    "body":{
        "messages":[
            {
            "@type":"/cosmos.bank.v1beta1.MsgSend",
            "from_address":"cosmos1pgdahdqhhckjuevfe7kmzfm94hr5vdw68mjt3l",
            "to_address":"cosmos1pgdahdqhhckjuevfe7kmzfm94hr5vdw68mjt3l",
            "amount":[
                {
                    "denom":"uatom",
                    "amount":100000
                }
            ]
            }
        ],
        "memo":""
    },
    "auth_info":{
        "signer_infos":[],
        "fee":{
            "amount":[
            {
                "denom":"uatom",
                "amount":5000
            }
            ],
            "gas_limit":200000
        }
    },
    "signatures":[]
}
"#;
