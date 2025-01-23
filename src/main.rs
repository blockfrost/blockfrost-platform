mod api;
mod blockfrost;
mod config;
mod db;
mod errors;
mod models;
mod payload;
mod schema;

use api::{register, root};
use axum::{
    routing::{get, post},
    Extension, Router,
};
use clap::Parser;
use colored::Colorize;
use config::{Args, Config};
use db::DB;
use dotenvy;
use std::net::SocketAddr;
use tracing_subscriber::fmt::format::Format;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let arguments = Args::parse();
    let config: Config = config::load_config(arguments.config);

    tracing_subscriber::fmt()
        .with_max_level(config.server.log_level)
        .event_format(
            Format::default()
                .with_ansi(true)
                .with_level(true)
                .with_target(false)
                .compact(),
        )
        .init();

    let pool = DB::new(&config.database.connection_string).await;
    let blockfrost_api = blockfrost::BlockfrostAPI::new(&config.blockfrost.project_id);

    let app = Router::new()
        .route("/", get(root::route))
        .route("/register", post(register::route))
        .layer(Extension(config.clone()))
        .layer(Extension(pool))
        .layer(Extension(blockfrost_api));

    let listener = tokio::net::TcpListener::bind(&config.server.address)
        .await
        .expect("Failed to bind to address");

    println!(
        "{}",
        format!(
            "\nAddress: 🌍 http://{}\n\
             Log Level: 📘 {}\n",
            config.server.address, config.server.log_level,
        )
        .white()
        .bold()
    );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap_or_else(|e| {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    });
}
