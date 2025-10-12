mod cache;
mod config;
mod error;
mod mcp;
mod slack;
mod tools;
mod utils;

use anyhow::Result;
use std::sync::Arc;
use tracing::error;

use crate::cache::SqliteCache;
use crate::config::Config;
use crate::mcp::server::McpServer;
use crate::slack::SlackClient;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize logging
    init_logging()?;

    // Load configuration
    let config_path = std::env::args().nth(1);

    // Use a shared SQLite database for all MCP instances
    let data_path = std::env::var("DATA_PATH").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        format!("{}/.mcp-slack", home)
    });

    // Ensure the directory exists
    if let Err(e) = std::fs::create_dir_all(&data_path) {
        error!("Failed to create data directory at {}: {}", data_path, e);
    }

    let db_path = format!("{}/cache.db", data_path);
    let config = Config::load(config_path.as_deref(), &data_path)?;

    // Initialize Slack client
    let slack_client = Arc::new(SlackClient::new(config.clone()));

    // Initialize SQLite cache
    let cache = Arc::new(SqliteCache::new(&db_path).await?);

    // Create and run MCP server with shared instances
    let mcp_server = McpServer::new(config, cache, slack_client).await?;

    // Set up graceful shutdown
    let shutdown_signal = tokio::signal::ctrl_c();

    // Run MCP server
    tokio::select! {
        result = mcp_server.run() => {
            match result {
                Ok(_) => {},
                Err(e) => error!("MCP server error: {}", e),
            }
        }
        _ = shutdown_signal => {
        }
    }

    // Cleanup
    Ok(())
}

fn init_logging() -> Result<()> {
    // Support both LOG_LEVEL and RUST_LOG environment variables
    let filter = if let Ok(rust_log) = std::env::var("RUST_LOG") {
        // Use RUST_LOG if set (allows module-specific logging)
        tracing_subscriber::EnvFilter::try_new(rust_log)
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"))
    } else if let Ok(log_level) = std::env::var("LOG_LEVEL") {
        // Use LOG_LEVEL for global level (error, warn, info, debug, trace)
        let level_str = match log_level.to_lowercase().as_str() {
            "trace" => "trace",
            "debug" => "debug",
            "info" => "info",
            "warn" | "warning" => "warn",
            "error" => "error",
            _ => "warn", // Default to WARN for invalid values
        };
        tracing_subscriber::EnvFilter::new(level_str)
    } else {
        // Default to WARN level
        tracing_subscriber::EnvFilter::new("warn")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr) // Write logs to stderr for MCP compatibility
        .compact() // Use compact format for cleaner output
        .with_target(false) // Don't show target module in logs
        .init();

    Ok(())
}
