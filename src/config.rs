use crate::constants::{DEFAULT_ETH_CHAIN_ID, DEFAULT_PORT};
use std::env;
use tracing::{error, info};

#[derive(Debug, Clone, Default)]
pub struct AwsCreds {
    pub access_key_id: String,
    pub secret_access_key: String,
}

#[derive(Debug, Clone, Default)]
pub struct AwsConfig {
    pub region: String,
    pub sign: AwsCreds,
    pub create: AwsCreds,
    pub delete: Option<AwsCreds>,
    pub list: Option<AwsCreds>,
    pub hash_type: HashType,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HashType {
    #[default]
    Sha256,
    Sha3_256,
    Keccak256,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub aws: AwsConfig,
    pub testing_mode: bool,
    pub aws_mode: bool,
    pub casper_mode: bool,
    pub ethereum_mode: bool,
    pub delete_mode: bool,
    pub list_mode: bool,
    pub eth_chain_id: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            aws: AwsConfig::default(),
            testing_mode: true, // TESTING_MODE true by default
            aws_mode: true,     // AWS_MODE true by default
            casper_mode: true,  // CASPER_MODE true by default
            ethereum_mode: false,
            delete_mode: false,
            list_mode: false,
            eth_chain_id: DEFAULT_ETH_CHAIN_ID,
        }
    }
}

impl Config {
    #[must_use]
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let testing_mode = env::var("TESTING_MODE")
            .map(|v| v == "true")
            .unwrap_or(true);

        let delete_mode = env::var("DELETE_MODE")
            .map(|v| v == "true")
            .unwrap_or(false);
        let list_mode = env::var("LIST_MODE").map(|v| v == "true").unwrap_or(false);

        let sign = get_creds("KMS_SIGN_ID", "KMS_SIGN_KEY");
        let create = get_creds("KMS_CREATE_ID", "KMS_CREATE_KEY");
        let delete = delete_mode.then(|| get_creds("KMS_DELETE_ID", "KMS_DELETE_KEY"));
        let list = list_mode.then(|| get_creds("KMS_LIST_ID", "KMS_LIST_KEY"));
        let aws_mode = env::var("AWS_MODE").map(|v| v == "true").unwrap_or(false);
        let casper_mode = env::var("CASPER_MODE")
            .map(|v| v == "true")
            .unwrap_or(false);
        let ethereum_mode = !casper_mode
            && env::var("ETHEREUM_MODE")
                .map(|v| v == "true")
                .unwrap_or(false);

        log_modes(&Modes {
            delete_mode,
            list_mode,
            aws_mode,
            testing_mode,
            casper_mode,
            ethereum_mode,
        });

        let hash_type = if casper_mode {
            HashType::Sha256
        } else if ethereum_mode {
            HashType::Keccak256
        } else {
            HashType::default()
        };

        Self {
            port: env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_PORT),
            aws: AwsConfig {
                region: env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".into()),
                sign,
                create,
                delete,
                list,
                hash_type,
            },
            aws_mode,
            casper_mode,
            ethereum_mode,
            testing_mode,
            delete_mode,
            list_mode,
            eth_chain_id: env::var("ETH_CHAIN_ID")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_ETH_CHAIN_ID),
        }
    }
}

fn get_creds(id_key: &str, secret_key: &str) -> AwsCreds {
    let id = env::var(id_key).unwrap_or_default();
    let secret = env::var(secret_key).unwrap_or_default();

    if id.is_empty() {
        error!("{id_key} env var is empty");
    }
    if secret.is_empty() {
        error!("{secret_key} env var is empty");
    }

    AwsCreds {
        access_key_id: id,
        secret_access_key: secret,
    }
}

#[allow(clippy::struct_excessive_bools)]
struct Modes {
    delete_mode: bool,
    list_mode: bool,
    aws_mode: bool,
    testing_mode: bool,
    casper_mode: bool,
    ethereum_mode: bool,
}

#[allow(clippy::cognitive_complexity)]
fn log_modes(modes: &Modes) {
    info!("testing_mode: {}", modes.testing_mode);
    info!("aws_mode: {}", modes.aws_mode);
    info!("delete_mode: {}", modes.delete_mode);
    info!("list_mode: {}", modes.list_mode);
    info!("casper_mode: {}", modes.casper_mode);
    info!("ethereum_mode: {}", modes.ethereum_mode);
}
