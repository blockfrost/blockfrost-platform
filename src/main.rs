mod api;
mod config;
mod db;
mod errors;
mod models;
mod schema;

use api::{register, root};
use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;
use colored::Colorize;
use config::{Args, Config};

#[tokio::main]
async fn main() {
    let arguments = Args::parse();
    let config: Config = config::load_config(arguments.config);
    let cloned_config = config.clone();

    tracing_subscriber::fmt()
        .with_max_level(config.server.log_level)
        .init();

    let pool = db::init_db(config.database.connection_string).await;

    let app = Router::new()
        .route("/", get(root::route))
        .route("/register", post(register::route))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind(config.server.address)
        .await
        .expect("Failed to bind to address");

    println!(
        "{}",
        format!(
            "\nAddress: 🌍 http://{}\n\
             Log Level: 📘 {}\n",
            cloned_config.server.address, cloned_config.server.log_level,
        )
        .white()
        .bold()
    );

    axum::serve(listener, app).await.unwrap_or_else(|e| {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    });
}
