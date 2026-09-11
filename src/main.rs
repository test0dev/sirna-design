use clap::Parser;
use sirna_design::api::{router, AppState, DEFAULT_LISTEN_ADDR};
use tokio::net::TcpListener;

#[derive(Debug, Parser)]
#[command(name = "sirna-design", about = "siRNA design engine")]
struct Args {
    #[arg(long, env = "LISTEN_ADDR", default_value = DEFAULT_LISTEN_ADDR)]
    listen: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let args = Args::parse();
    let state = match AppState::from_env() {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("failed to build API state: {e}");
            std::process::exit(1);
        }
    };
    let app = router(state);
    let listener = match TcpListener::bind(&args.listen).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("bind {}: {e}", args.listen);
            std::process::exit(1);
        }
    };
    tracing::info!("sirna-design listening on {}", args.listen);
    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!("server: {e}");
        std::process::exit(1);
    }
}
