use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "sirna-design", about = "siRNA design engine")]
struct Args {
    #[arg(long, default_value = "0.0.0.0:8080")]
    listen: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let args = Args::parse();
    tracing::info!("sirna-design scaffold listening placeholder on {}", args.listen);
    // HTTP API lands in a later step.
}
