use anyhow::Result;
use goose_mcp::{
    ComputerControllerRouter, Developer, DeveloperRouter, GoogleDriveRouter, JetBrainsRouter,
    MemoryRouter, TutorialRouter,
};
use mcp_server::router::RouterService;
use mcp_server::{BoundedService, ByteTransport, Server};
use tokio::io::{stdin, stdout};

pub async fn run_server(name: &str) -> Result<()> {
    // Initialize logging
    //crate::logging::setup_logging(Some(&format!("mcp-{name}")), None)?;

    tracing::info!("Starting MCP server");

    if name == "developer_rmcp" {
        use rmcp::{handler::server::ServerHandlerService, service::serve_server};
        use tokio::io::{stdin, stdout};

        let service = ServerHandlerService::new(Developer::new());

        let transport = (stdin(), stdout());

        let server = serve_server(service, transport).await?;

        server.waiting().await?;
        Ok(())
    } else {
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

        // Create and run the server
        let server =
            Server::new(router.unwrap_or_else(|| panic!("Unknown server requested {}", name)));
        let transport = ByteTransport::new(stdin(), stdout());

        tracing::info!("Server initialized and ready to handle requests");
        Ok(server.run(transport).await?)
    }
}
