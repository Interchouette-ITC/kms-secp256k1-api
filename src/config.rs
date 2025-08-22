use crate::constants::{
    DEFAULT_APP_ADDR, DEFAULT_APP_PORT, DEFAULT_COSMOS_CHAIN_ID, DEFAULT_COSMOS_HRP,
    DEFAULT_COSMOS_REST_URL, DEFAULT_ETH_CHAIN_ID,
};
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
    Keccak256,
    // Sha3_256,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum BlockchainMode {
    #[default]
    Casper,
    Ethereum,
    Cosmos,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct Config {
    blockchain_mode: BlockchainMode,
    addr: String,
    port: u16,
    aws: AwsConfig,
    testing_mode: bool,
    aws_mode: bool,
    delete_mode: bool,
    list_mode: bool,
    eth_chain_id: u8,
    cosmos_hrp: String,
    cosmos_rest_url: String,
    cosmos_chain_id: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            blockchain_mode: BlockchainMode::default(), // CASPER MODE by default
            port: DEFAULT_APP_PORT,
            addr: DEFAULT_APP_ADDR.to_string(),
            aws: AwsConfig::default(),
            testing_mode: true, // TESTING_MODE true by default
            aws_mode: false,
            delete_mode: false,
            list_mode: false,
            eth_chain_id: DEFAULT_ETH_CHAIN_ID,
            cosmos_hrp: DEFAULT_COSMOS_HRP.to_string(),
            cosmos_rest_url: DEFAULT_COSMOS_REST_URL.to_string(),
            cosmos_chain_id: DEFAULT_COSMOS_CHAIN_ID.to_string(),
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
        let aws_mode = env::var("AWS_MODE").map(|v| v == "true").unwrap_or(true);

        let blockchain_mode = match env::var("BLOCKCHAIN_MODE")
            .unwrap_or_else(|_| format!("{:?}", BlockchainMode::default()))
            .to_lowercase()
            .as_str()
        {
            "casper" => BlockchainMode::Casper,
            "ethereum" => BlockchainMode::Ethereum,
            "cosmos" => BlockchainMode::Cosmos,
            _ => BlockchainMode::default(),
        };

        let hash_type = match blockchain_mode {
            BlockchainMode::Casper | BlockchainMode::Cosmos => HashType::Sha256,
            BlockchainMode::Ethereum => HashType::Keccak256,
        };

        log_modes(&Modes {
            delete_mode,
            list_mode,
            aws_mode,
            testing_mode,
            blockchain_mode: blockchain_mode.clone(),
            hash_type,
        });

        Self {
            blockchain_mode,
            port: env::var("APP_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_APP_PORT),
            addr: env::var("APP_ADDR")
                .ok()
                .unwrap_or_else(|| DEFAULT_APP_ADDR.to_string()),
            aws: AwsConfig {
                region: env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".into()),
                sign,
                create,
                delete,
                list,
                hash_type,
            },
            aws_mode,
            testing_mode,
            delete_mode,
            list_mode,
            eth_chain_id: env::var("ETH_CHAIN_ID")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_ETH_CHAIN_ID),
            cosmos_hrp: env::var("COSMOS_HRP")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(|| DEFAULT_COSMOS_HRP.to_string()),
            cosmos_rest_url: env::var("COSMOS_REST_URL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(|| DEFAULT_COSMOS_REST_URL.to_string()),
            cosmos_chain_id: env::var("COSMOS_CHAIN_ID")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(|| DEFAULT_COSMOS_CHAIN_ID.to_string()),
        }
    }

    #[must_use]
    pub const fn is_testing_mode(&self) -> bool {
        self.testing_mode
    }

    #[must_use]
    pub fn is_ethereum_mode(&self) -> bool {
        self.blockchain_mode == BlockchainMode::Ethereum
    }

    #[must_use]
    pub fn is_casper_mode(&self) -> bool {
        self.blockchain_mode == BlockchainMode::Casper
    }

    #[must_use]
    pub fn is_cosmos_mode(&self) -> bool {
        self.blockchain_mode == BlockchainMode::Cosmos
    }

    #[must_use]
    pub const fn is_delete_mode(&self) -> bool {
        self.delete_mode
    }

    #[must_use]
    pub const fn is_list_mode(&self) -> bool {
        self.list_mode
    }

    #[must_use]
    pub const fn is_aws_mode(&self) -> bool {
        self.aws_mode
    }

    #[must_use]
    pub fn get_aws_config(&self) -> AwsConfig {
        self.aws.clone()
    }

    #[must_use]
    pub const fn get_port(&self) -> u16 {
        self.port
    }

    #[must_use]
    pub fn get_addr(&self) -> String {
        self.addr.clone()
    }

    #[must_use]
    pub const fn get_eth_chain_id(&self) -> u8 {
        self.eth_chain_id
    }

    #[must_use]
    pub fn get_cosmos_hrp(&self) -> String {
        self.cosmos_hrp.clone()
    }

    #[must_use]
    pub fn get_cosmos_rest_url(&self) -> String {
        self.cosmos_rest_url.clone()
    }

    #[must_use]
    pub fn get_cosmos_chain_id(&self) -> String {
        self.cosmos_chain_id.clone()
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
    blockchain_mode: BlockchainMode,
    hash_type: HashType,
}

#[allow(clippy::cognitive_complexity)]
fn log_modes(modes: &Modes) {
    info!("testing_mode: {}", modes.testing_mode);
    info!("aws_mode: {}", modes.aws_mode);
    info!("delete_mode: {}", modes.delete_mode);
    info!("list_mode: {}", modes.list_mode);
    info!("blockchain_mode: {:?}", modes.blockchain_mode);
    info!("hash_type: {:?}", modes.hash_type);
}

#[derive(Debug, Default)]
pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn with_blockchain_mode(mut self, mode: BlockchainMode) -> Self {
        self.config.blockchain_mode = mode;
        self
    }

    #[must_use]
    pub const fn with_casper_mode(mut self) -> Self {
        self.config.blockchain_mode = BlockchainMode::Casper;
        self
    }

    #[must_use]
    pub const fn with_ethereum_mode(mut self) -> Self {
        self.config.blockchain_mode = BlockchainMode::Ethereum;
        self
    }

    #[must_use]
    pub const fn with_cosmos_mode(mut self) -> Self {
        self.config.blockchain_mode = BlockchainMode::Cosmos;
        self
    }

    #[must_use]
    pub const fn with_testing_mode(mut self, enabled: bool) -> Self {
        self.config.testing_mode = enabled;
        self
    }

    #[must_use]
    pub const fn with_delete_mode(mut self, enabled: bool) -> Self {
        self.config.delete_mode = enabled;
        self
    }

    #[must_use]
    pub const fn with_list_mode(mut self, enabled: bool) -> Self {
        self.config.list_mode = enabled;
        self
    }

    #[must_use]
    pub const fn with_aws_mode(mut self, enabled: bool) -> Self {
        self.config.aws_mode = enabled;
        self
    }

    #[must_use]
    pub const fn with_port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    #[must_use]
    pub const fn with_eth_chain_id(mut self, id: u8) -> Self {
        self.config.eth_chain_id = id;
        self
    }

    #[must_use]
    pub fn with_aws_config(mut self, aws: AwsConfig) -> Self {
        self.config.aws = aws;
        self
    }

    #[must_use]
    pub fn build(self) -> Config {
        self.config
    }
}
