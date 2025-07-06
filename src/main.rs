use kms_secp256k1_api::{config::Config, run_server};
use once_cell::sync::OnceCell;
use tracing_subscriber::{EnvFilter, fmt};

static TRACING_INIT: OnceCell<()> = OnceCell::new();

fn init_tracing() {
    TRACING_INIT.get_or_init(|| {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        fmt().with_env_filter(filter).init();
    });
}

#[tokio::main]
async fn main() {
    init_tracing();

    let config = Config::from_env();

    if let Err(e) = run_server(config).await {
        eprintln!("Server error: {e}");
        std::process::exit(1);
    }
}
