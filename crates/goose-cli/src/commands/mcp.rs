use anyhow::Result;
use goose_mcp::{
    ComputerControllerRouter, DeveloperRouter, GoogleDriveRouter, JetBrainsRouter, MemoryRouter,
    TutorialRouter,
};
use mcp_server::router::RouterService;
use mcp_server::{BoundedService, ByteTransport, Server};
use tokio::io::{stdin, stdout};

use std::sync::Arc;
use tokio::sync::Notify;

pub async fn run_server(name: &str) -> Result<()> {
    // Initialize logging
    crate::logging::setup_logging(Some(&format!("mcp-{name}")), None)?;

    tracing::info!("Starting MCP server");

    let router: Option<Box<dyn BoundedService>> = match name {
        "developer" => Some(Box::new(RouterService(DeveloperRouter::new()))),
        "computercontroller" => Some(Box::new(RouterService(ComputerControllerRouter::new()))),
        "jetbrains" => Some(Box::new(RouterService(JetBrainsRouter::new()))),
        "google_drive" | "googledrive" => {
            let router = GoogleDriveRouter::new().await;
            Some(Box::new(RouterService(router)))
        }
        "memory" => Some(Box::new(RouterService(MemoryRouter::new()))),
        "tutorial" => Some(Box::new(RouterService(TutorialRouter::new()))),
        _ => None,
    };

    // Create shutdown notification channel
    let shutdown = Arc::new(Notify::new());
    let shutdown_clone = shutdown.clone();

    // Spawn signal handler
    tokio::spawn(async move {
        crate::signal::shutdown_signal().await;
        shutdown_clone.notify_one();
    });

    // Create and run the server
    let server = Server::new(router.unwrap_or_else(|| panic!("Unknown server requested {}", name)));
    let transport = ByteTransport::new(stdin(), stdout());

    tracing::info!("Server initialized and ready to handle requests");

    tokio::select! {
        result = server.run(transport) => {
            tracing::info!("Server completed normally");
            Ok(result?)
        }
        _ = shutdown.notified() => {
            tracing::info!("Received shutdown signal");
            
            // On Unix systems, kill the entire process group
            #[cfg(unix)]
            unsafe {
                let pgid = libc::getpgid(0);
                if pgid > 0 {
                    tracing::debug!("Sending SIGTERM to process group {}", pgid);
                    libc::kill(-pgid, libc::SIGTERM);
                }
            }
            
            Ok(())
        }
    }
}
