use crate::constants::{
    AWS_KMS_ENDPOINT_PATTERN, DEFAULT_APP_ADDR, DEFAULT_APP_PORT, DEFAULT_AWS_REGION,
    DEFAULT_COSMOS_CHAIN_ID, DEFAULT_COSMOS_HRP, DEFAULT_COSMOS_REST_URL, DEFAULT_ETH_CHAIN_ID,
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
    pub endpoint: String,
    pub sign: AwsCreds,
    pub create: AwsCreds,
    pub delete: Option<AwsCreds>,
    pub list: Option<AwsCreds>,
    pub hash_type: HashType,
    /// When true, AWS SDK clients use the default credential chain (task role / instance profile).
    /// When false, clients use the static `KMS_*` access keys in this struct.
    pub use_default_credentials: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HashType {
    #[default]
    Sha256,
    Keccak256,
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
        match Self::try_from_env() {
            Ok(config) => config,
            Err(err) => {
                error!("{err}");
                std::process::exit(1);
            }
        }
    }

    /// Loads config from the process environment.
    ///
    /// # Errors
    ///
    /// Returns an error when `KMS_*` pairs are mixed (one side empty), when
    /// `DELETE_MODE`/`LIST_MODE` require static keys that are missing, or when
    /// the default credential chain is selected but `AWS_ACCESS_KEY_ID` /
    /// `AWS_SECRET_ACCESS_KEY` would shadow the role.
    pub fn try_from_env() -> Result<Self, String> {
        maybe_load_dotenv();

        let testing_mode = env::var("TESTING_MODE").map_or(true, |v| v == "true");
        let delete_mode = env::var("DELETE_MODE").is_ok_and(|v| v == "true");
        let list_mode = env::var("LIST_MODE").is_ok_and(|v| v == "true");
        let aws_mode = env::var("AWS_MODE").map_or(true, |v| v == "true");

        let resolved = resolve_aws_credentials(delete_mode, list_mode)?;

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
            use_default_credentials: resolved.use_default_credentials,
            blockchain_mode: blockchain_mode.clone(),
            hash_type,
        });

        let region = env::var("AWS_REGION").unwrap_or_else(|_| DEFAULT_AWS_REGION.into());
        let endpoint = env::var("AWS_ENDPOINT")
            .unwrap_or_else(|_| AWS_KMS_ENDPOINT_PATTERN.replace("{}", &region));

        Ok(Self {
            blockchain_mode,
            port: env::var("APP_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_APP_PORT),
            addr: env::var("APP_ADDR")
                .ok()
                .unwrap_or_else(|| DEFAULT_APP_ADDR.to_string()),
            aws: AwsConfig {
                region,
                endpoint,
                sign: resolved.sign,
                create: resolved.create,
                delete: resolved.delete,
                list: resolved.list,
                hash_type,
                use_default_credentials: resolved.use_default_credentials,
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
        })
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
    pub fn get_blockchain_mode(&self) -> BlockchainMode {
        self.blockchain_mode.clone()
    }

    /// Panics if this config's blockchain mode was not compiled into the binary.
    ///
    /// Enable the matching Cargo feature (`casper`, `ethereum`, `cosmos`) or `--features all`.
    #[allow(clippy::missing_const_for_fn)] // body is empty when all chain features are enabled
    pub fn ensure_blockchain_feature(&self) {
        #[cfg(not(feature = "casper"))]
        if self.is_casper_mode() {
            panic!(
                "Blockchain mode Casper requires --features casper (or --features all); this binary was built without it"
            );
        }
        #[cfg(not(feature = "ethereum"))]
        if self.is_ethereum_mode() {
            panic!(
                "Blockchain mode Ethereum requires --features ethereum (or --features all); this binary was built without it"
            );
        }
        #[cfg(not(feature = "cosmos"))]
        if self.is_cosmos_mode() {
            panic!(
                "Blockchain mode Cosmos requires --features cosmos (or --features all); this binary was built without it"
            );
        }
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

/// Loads `.env` unless `DOTENV_DISABLE` is set in the process environment.
pub fn maybe_load_dotenv() {
    if env::var("DOTENV_DISABLE").is_ok() {
        info!("DOTENV_DISABLE set; skipping .env load");
        return;
    }
    dotenvy::dotenv().ok();
}

/// Returns whether `DOTENV_DISABLE` is set (presence only; value ignored).
#[must_use]
pub fn dotenv_disabled() -> bool {
    env::var("DOTENV_DISABLE").is_ok()
}

/// Reads an access-key / secret-key pair from the environment.
///
/// # Errors
///
/// Returns an error when exactly one of the two variables is non-empty.
pub fn read_cred_pair(id_key: &str, secret_key: &str) -> Result<Option<AwsCreds>, String> {
    let id = env::var(id_key).unwrap_or_default();
    let secret = env::var(secret_key).unwrap_or_default();
    match (id.is_empty(), secret.is_empty()) {
        (true, true) => Ok(None),
        (false, false) => Ok(Some(AwsCreds {
            access_key_id: id,
            secret_access_key: secret,
        })),
        (true, false) => Err(format!(
            "{id_key} is empty but {secret_key} is set; both must be set or both unset"
        )),
        (false, true) => Err(format!(
            "{secret_key} is empty but {id_key} is set; both must be set or both unset"
        )),
    }
}

/// Fails when standard AWS access-key env vars would shadow the default credential chain.
///
/// # Errors
///
/// Returns an error if `AWS_ACCESS_KEY_ID` or `AWS_SECRET_ACCESS_KEY` is present.
pub fn refuse_shadowing_aws_env_creds() -> Result<(), String> {
    let access = env::var("AWS_ACCESS_KEY_ID").ok().filter(|s| !s.is_empty());
    let secret = env::var("AWS_SECRET_ACCESS_KEY")
        .ok()
        .filter(|s| !s.is_empty());
    if access.is_some() || secret.is_some() {
        return Err(
            "default credential chain selected (KMS_* keys unset), but AWS_ACCESS_KEY_ID and/or AWS_SECRET_ACCESS_KEY are set; unset them so the task role / instance profile is used"
                .into(),
        );
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct ResolvedAwsCredentials {
    create: AwsCreds,
    sign: AwsCreds,
    delete: Option<AwsCreds>,
    list: Option<AwsCreds>,
    use_default_credentials: bool,
}

fn resolve_aws_credentials(
    delete_mode: bool,
    list_mode: bool,
) -> Result<ResolvedAwsCredentials, String> {
    let create = read_cred_pair("KMS_CREATE_ID", "KMS_CREATE_KEY")?;
    let sign = read_cred_pair("KMS_SIGN_ID", "KMS_SIGN_KEY")?;
    let delete_pair = read_cred_pair("KMS_DELETE_ID", "KMS_DELETE_KEY")?;
    let list_pair = read_cred_pair("KMS_LIST_ID", "KMS_LIST_KEY")?;

    match (create, sign) {
        (None, None) => {
            refuse_shadowing_aws_env_creds()?;
            if delete_pair.is_some() {
                return Err(
                    "KMS_DELETE_* is set but KMS_CREATE_*/KMS_SIGN_* are unset; use the default credential chain without static delete keys, or set all required static keys"
                        .into(),
                );
            }
            if list_pair.is_some() {
                return Err(
                    "KMS_LIST_* is set but KMS_CREATE_*/KMS_SIGN_* are unset; use the default credential chain without static list keys, or set all required static keys"
                        .into(),
                );
            }
            Ok(ResolvedAwsCredentials {
                create: AwsCreds::default(),
                sign: AwsCreds::default(),
                delete: delete_mode.then(AwsCreds::default),
                list: list_mode.then(AwsCreds::default),
                use_default_credentials: true,
            })
        }
        (Some(create), Some(sign)) => {
            let delete = if delete_mode {
                Some(delete_pair.ok_or_else(|| {
                    "DELETE_MODE=true requires KMS_DELETE_ID and KMS_DELETE_KEY".to_string()
                })?)
            } else {
                None
            };
            let list = if list_mode {
                Some(list_pair.ok_or_else(|| {
                    "LIST_MODE=true requires KMS_LIST_ID and KMS_LIST_KEY".to_string()
                })?)
            } else {
                None
            };
            Ok(ResolvedAwsCredentials {
                create,
                sign,
                delete,
                list,
                use_default_credentials: false,
            })
        }
        (None, Some(_)) | (Some(_), None) => Err(
            "KMS_CREATE_* and KMS_SIGN_* must both be set (static keys) or both unset (default credential chain)"
                .into(),
        ),
    }
}

#[allow(clippy::struct_excessive_bools)]
struct Modes {
    delete_mode: bool,
    list_mode: bool,
    aws_mode: bool,
    testing_mode: bool,
    use_default_credentials: bool,
    blockchain_mode: BlockchainMode,
    hash_type: HashType,
}

fn log_modes(modes: &Modes) {
    info!("testing_mode: {}", modes.testing_mode);
    info!("aws_mode: {}", modes.aws_mode);
    info!("delete_mode: {}", modes.delete_mode);
    info!("list_mode: {}", modes.list_mode);
    info!("use_default_credentials: {}", modes.use_default_credentials);
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
        self.config.aws.hash_type = match mode {
            BlockchainMode::Ethereum => HashType::Keccak256,
            BlockchainMode::Casper | BlockchainMode::Cosmos => HashType::Sha256,
        };
        self.config.blockchain_mode = mode;
        self
    }

    #[must_use]
    pub const fn with_casper_mode(mut self) -> Self {
        self.config.blockchain_mode = BlockchainMode::Casper;
        self.config.aws.hash_type = HashType::Sha256;
        self
    }

    #[must_use]
    pub const fn with_ethereum_mode(mut self) -> Self {
        self.config.blockchain_mode = BlockchainMode::Ethereum;
        self.config.aws.hash_type = HashType::Keccak256;
        self
    }

    #[must_use]
    pub const fn with_cosmos_mode(mut self) -> Self {
        self.config.blockchain_mode = BlockchainMode::Cosmos;
        self.config.aws.hash_type = HashType::Sha256;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn clear_kms_and_aws_key_env() {
        for key in [
            "KMS_CREATE_ID",
            "KMS_CREATE_KEY",
            "KMS_SIGN_ID",
            "KMS_SIGN_KEY",
            "KMS_DELETE_ID",
            "KMS_DELETE_KEY",
            "KMS_LIST_ID",
            "KMS_LIST_KEY",
            "AWS_ACCESS_KEY_ID",
            "AWS_SECRET_ACCESS_KEY",
            "DOTENV_DISABLE",
        ] {
            // SAFETY: tests hold ENV_LOCK; only this process mutates these keys.
            unsafe { env::remove_var(key) };
        }
    }

    #[test]
    fn dotenv_disabled_detects_presence() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_kms_and_aws_key_env();
        assert!(!dotenv_disabled());
        // SAFETY: see clear_kms_and_aws_key_env
        unsafe { env::set_var("DOTENV_DISABLE", "1") };
        assert!(dotenv_disabled());
        unsafe { env::remove_var("DOTENV_DISABLE") };
    }

    #[test]
    fn read_cred_pair_empty_full_and_mixed() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_kms_and_aws_key_env();

        assert!(
            read_cred_pair("KMS_CREATE_ID", "KMS_CREATE_KEY")
                .unwrap()
                .is_none()
        );

        unsafe {
            env::set_var("KMS_CREATE_ID", "akid");
            env::set_var("KMS_CREATE_KEY", "secret");
        }
        let pair = read_cred_pair("KMS_CREATE_ID", "KMS_CREATE_KEY")
            .unwrap()
            .expect("pair");
        assert_eq!(pair.access_key_id, "akid");
        assert_eq!(pair.secret_access_key, "secret");

        unsafe { env::remove_var("KMS_CREATE_KEY") };
        let err = read_cred_pair("KMS_CREATE_ID", "KMS_CREATE_KEY").unwrap_err();
        assert!(err.contains("both must be set"));
        clear_kms_and_aws_key_env();
    }

    #[test]
    fn resolve_empty_keys_uses_default_chain() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_kms_and_aws_key_env();
        let resolved = resolve_aws_credentials(true, true).unwrap();
        assert!(resolved.use_default_credentials);
        assert!(resolved.delete.is_some());
        assert!(resolved.list.is_some());
        assert!(resolved.create.access_key_id.is_empty());
    }

    #[test]
    fn resolve_static_keys_path() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_kms_and_aws_key_env();
        unsafe {
            env::set_var("KMS_CREATE_ID", "c");
            env::set_var("KMS_CREATE_KEY", "ck");
            env::set_var("KMS_SIGN_ID", "s");
            env::set_var("KMS_SIGN_KEY", "sk");
            env::set_var("KMS_DELETE_ID", "d");
            env::set_var("KMS_DELETE_KEY", "dk");
        }
        let resolved = resolve_aws_credentials(true, false).unwrap();
        assert!(!resolved.use_default_credentials);
        assert_eq!(resolved.create.access_key_id, "c");
        assert_eq!(resolved.sign.access_key_id, "s");
        assert_eq!(resolved.delete.as_ref().unwrap().access_key_id, "d");
        assert!(resolved.list.is_none());
        clear_kms_and_aws_key_env();
    }

    #[test]
    fn resolve_refuses_aws_access_key_on_default_chain() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_kms_and_aws_key_env();
        unsafe { env::set_var("AWS_ACCESS_KEY_ID", "AKIA...") };
        let err = resolve_aws_credentials(false, false).unwrap_err();
        assert!(err.contains("AWS_ACCESS_KEY_ID"));
        clear_kms_and_aws_key_env();
    }

    #[test]
    fn resolve_refuses_create_sign_mismatch() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_kms_and_aws_key_env();
        unsafe {
            env::set_var("KMS_CREATE_ID", "c");
            env::set_var("KMS_CREATE_KEY", "ck");
        }
        let err = resolve_aws_credentials(false, false).unwrap_err();
        assert!(err.contains("both be set") || err.contains("both unset"));
        clear_kms_and_aws_key_env();
    }

    #[test]
    fn resolve_delete_mode_requires_static_delete_keys() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_kms_and_aws_key_env();
        unsafe {
            env::set_var("KMS_CREATE_ID", "c");
            env::set_var("KMS_CREATE_KEY", "ck");
            env::set_var("KMS_SIGN_ID", "s");
            env::set_var("KMS_SIGN_KEY", "sk");
        }
        let err = resolve_aws_credentials(true, false).unwrap_err();
        assert!(err.contains("KMS_DELETE"));
        clear_kms_and_aws_key_env();
    }

    #[test]
    fn refuse_shadowing_ok_when_unset() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_kms_and_aws_key_env();
        refuse_shadowing_aws_env_creds().unwrap();
    }
}
