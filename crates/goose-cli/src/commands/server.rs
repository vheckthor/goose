use anyhow::Result;
use developer::DeveloperRouter;
use nondeveloper::NonDeveloperRouter;
use mcp_server::router::RouterService;
use mcp_server::{ByteTransport, Server, BoxError};
use tokio::io::{stdin, stdout};
use tower_service::Service;
use mcp_core::protocol::{JsonRpcRequest, JsonRpcResponse};
use std::future::Future;
use std::pin::Pin;

pub enum UnifiedRouterService {
    Developer(RouterService<DeveloperRouter>),
    NonDeveloper(RouterService<NonDeveloperRouter>),
}

impl UnifiedRouterService {
    pub fn into_router_service(
        self,
    ) -> Box<dyn Service<
        JsonRpcRequest,
        Response = JsonRpcResponse,
        Error = BoxError,
        Future = Pin<Box<dyn Future<Output = Result<JsonRpcResponse, BoxError>> + Send>>
    > + Send> {
        match self {
            UnifiedRouterService::Developer(service) => Box::new(service),
            UnifiedRouterService::NonDeveloper(service) => Box::new(service),
        }
    }
}

pub async fn run_server(name: &str) -> Result<()> {
    tracing::info!("Starting MCP server");

    let router = match name {
        "developer" => UnifiedRouterService::Developer(RouterService(DeveloperRouter::new())),
        "nondeveloper" => UnifiedRouterService::NonDeveloper(RouterService(NonDeveloperRouter::new())),
        _ => panic!("Unknown server requested {}", name),
    };

    // Create and run the server
    let server = Server::new(router.into_router_service());
    let transport = ByteTransport::new(stdin(), stdout());

    tracing::info!("Server initialized and ready to handle requests");
    Ok(server.run(transport).await?)
}
