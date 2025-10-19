use anyhow::Result;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::web::routes;

pub async fn run_server(host: &str, port: u16) -> Result<()> {
    // Create the router with all routes
    let app = routes::create_routes()
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    // Bind to the address
    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Print startup message
    println!("🌐 RetroChat Web UI running at http://{addr}");
    println!("📊 API available at http://{addr}/api");
    println!("🏥 Health check: http://{addr}/api/health");
    println!();
    println!("Press Ctrl+C to stop the server");

    // Start the server
    axum::serve(listener, app).await?;

    Ok(())
}
