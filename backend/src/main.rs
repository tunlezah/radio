mod api;
mod casting;
mod dsp;
mod services;
mod streaming;
mod system;

use actix_cors::Cors;
use actix_files as fs;
use actix_web::{web, App, HttpServer};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

use crate::services::app_state::AppState;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!("Starting DAB+ Radio Backend v1.0.0");

    let app_state = Arc::new(AppState::new());

    // Start background services
    let state_clone = app_state.clone();
    tokio::spawn(async move {
        services::signal_monitor::run_signal_monitor(state_clone).await;
    });

    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    tracing::info!("Listening on {}", bind_addr);

    let state = web::Data::from(app_state);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .app_data(state.clone())
            .configure(api::rest::configure)
            .configure(api::websocket::configure)
            .service(fs::Files::new("/", "./static").index_file("index.html"))
    })
    .bind(&bind_addr)?
    .run()
    .await?;

    Ok(())
}
