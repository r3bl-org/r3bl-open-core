use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};
use rustmcp::server::RustMcpServer;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the rust-analyzer integration
    let mut rust_server = RustMcpServer::new();
    rust_server.start().await?;

    // Note: The #[tool] macros generate additional tools beyond our manual list
    eprintln!("Starting Rust MCP Server");
    eprintln!("Server running on stdio transport...");

    // Start the MCP server using the ServiceExt trait
    let service = rust_server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
