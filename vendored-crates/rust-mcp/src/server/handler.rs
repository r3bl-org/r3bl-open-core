use anyhow::Result;
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::{ErrorData as McpError, *},
    tool, tool_handler, tool_router,
};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::analyzer::RustAnalyzerClient;
use crate::server::parameters::*;
use crate::tools::{execute_tool, get_tools};

#[derive(Clone)]
pub struct RustMcpServer {
    analyzer: Arc<Mutex<RustAnalyzerClient>>,
    tool_router: ToolRouter<RustMcpServer>,
}

impl Default for RustMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl RustMcpServer {
    pub fn new() -> Self {
        Self {
            analyzer: Arc::new(Mutex::new(RustAnalyzerClient::new())),
            tool_router: Self::tool_router(),
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        let mut analyzer = self.analyzer.lock().await;
        analyzer.start().await
    }

    pub fn list_tools(&self) -> Vec<crate::tools::ToolDefinition> {
        get_tools()
    }

    pub async fn call_tool(&mut self, name: &str, args: Value) -> Result<crate::tools::ToolResult> {
        let mut analyzer = self.analyzer.lock().await;
        execute_tool(name, args, &mut analyzer).await
    }

    #[tool(description = "Find the definition of a symbol at a given position")]
    async fn find_definition(
        &self,
        Parameters(FindDefinitionParams {
            file_path,
            line,
            character,
        }): Parameters<FindDefinitionParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "file_path": file_path,
            "line": line,
            "character": character
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("find_definition", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "No definition found",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Find all references to a symbol at a given position")]
    async fn find_references(
        &self,
        Parameters(FindReferencesParams {
            file_path,
            line,
            character,
        }): Parameters<FindReferencesParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "file_path": file_path,
            "line": line,
            "character": character
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("find_references", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "No references found",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Get compiler diagnostics for a file")]
    async fn get_diagnostics(
        &self,
        Parameters(GetDiagnosticsParams { file_path }): Parameters<GetDiagnosticsParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "file_path": file_path
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("get_diagnostics", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "No diagnostics found",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Search for symbols in the workspace")]
    async fn workspace_symbols(
        &self,
        Parameters(WorkspaceSymbolsParams { query }): Parameters<WorkspaceSymbolsParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "query": query
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("workspace_symbols", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "No symbols found",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Rename a symbol with scope awareness")]
    async fn rename_symbol(
        &self,
        Parameters(RenameSymbolParams {
            file_path,
            line,
            character,
            new_name,
        }): Parameters<RenameSymbolParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "file_path": file_path,
            "line": line,
            "character": character,
            "new_name": new_name
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("rename_symbol", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "Rename operation completed",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Apply rustfmt formatting to a file")]
    async fn format_code(
        &self,
        Parameters(FormatCodeParams { file_path }): Parameters<FormatCodeParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "file_path": file_path
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("format_code", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "Format operation completed",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Parse and analyze Cargo.toml file")]
    async fn analyze_manifest(
        &self,
        Parameters(AnalyzeManifestParams { manifest_path }): Parameters<AnalyzeManifestParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "manifest_path": manifest_path
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("analyze_manifest", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "Analysis completed",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Execute cargo check and parse errors")]
    async fn run_cargo_check(
        &self,
        Parameters(RunCargoCheckParams { workspace_path }): Parameters<RunCargoCheckParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "workspace_path": workspace_path
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("run_cargo_check", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "Cargo check completed",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Extract selected code into a new function")]
    async fn extract_function(
        &self,
        Parameters(ExtractFunctionParams {
            file_path,
            start_line,
            start_character,
            end_line,
            end_character,
            function_name,
        }): Parameters<ExtractFunctionParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "file_path": file_path,
            "start_line": start_line,
            "start_character": start_character,
            "end_line": end_line,
            "end_character": end_character,
            "function_name": function_name
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("extract_function", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "Function extracted successfully",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Generate a struct with specified fields and derives")]
    async fn generate_struct(
        &self,
        Parameters(GenerateStructParams {
            struct_name,
            fields,
            derives,
            file_path,
        }): Parameters<GenerateStructParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "struct_name": struct_name,
            "fields": fields,
            "derives": derives,
            "file_path": file_path
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("generate_struct", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "Struct generated successfully",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Generate an enum with specified variants and derives")]
    async fn generate_enum(
        &self,
        Parameters(GenerateEnumParams {
            enum_name,
            variants,
            derives,
            file_path,
        }): Parameters<GenerateEnumParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "enum_name": enum_name,
            "variants": variants,
            "derives": derives,
            "file_path": file_path
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("generate_enum", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "Enum generated successfully",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Generate a trait implementation for a struct")]
    async fn generate_trait_impl(
        &self,
        Parameters(GenerateTraitImplParams {
            trait_name,
            struct_name,
            file_path,
        }): Parameters<GenerateTraitImplParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "trait_name": trait_name,
            "struct_name": struct_name,
            "file_path": file_path
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("generate_trait_impl", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "Trait implementation generated successfully",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Generate unit tests for a function")]
    async fn generate_tests(
        &self,
        Parameters(GenerateTestsParams {
            target_function,
            file_path,
            test_cases,
        }): Parameters<GenerateTestsParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "target_function": target_function,
            "file_path": file_path,
            "test_cases": test_cases
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("generate_tests", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "Tests generated successfully",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Inline a function call at specified position")]
    async fn inline_function(
        &self,
        Parameters(InlineFunctionParams {
            file_path,
            line,
            character,
        }): Parameters<InlineFunctionParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "file_path": file_path,
            "line": line,
            "character": character
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("inline_function", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "Function inlined successfully",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Change the signature of a function")]
    async fn change_signature(
        &self,
        Parameters(ChangeSignatureParams {
            file_path,
            line,
            character,
            new_signature,
        }): Parameters<ChangeSignatureParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "file_path": file_path,
            "line": line,
            "character": character,
            "new_signature": new_signature
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("change_signature", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "Signature changed successfully",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Organize and sort import statements in a file")]
    async fn organize_imports(
        &self,
        Parameters(OrganizeImportsParams { file_path }): Parameters<OrganizeImportsParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "file_path": file_path
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("organize_imports", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "Imports organized successfully",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Apply clippy lint suggestions to improve code quality")]
    async fn apply_clippy_suggestions(
        &self,
        Parameters(ApplyClippySuggestionsParams { file_path }): Parameters<
            ApplyClippySuggestionsParams,
        >,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "file_path": file_path
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("apply_clippy_suggestions", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "Clippy suggestions applied successfully",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Validate and suggest lifetime annotations")]
    async fn validate_lifetimes(
        &self,
        Parameters(ValidateLifetimesParams { file_path }): Parameters<ValidateLifetimesParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "file_path": file_path
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("validate_lifetimes", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "Lifetimes validated successfully",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Get type hierarchy for a symbol at specified position")]
    async fn get_type_hierarchy(
        &self,
        Parameters(GetTypeHierarchyParams {
            file_path,
            line,
            character,
        }): Parameters<GetTypeHierarchyParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "file_path": file_path,
            "line": line,
            "character": character
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("get_type_hierarchy", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "Type hierarchy retrieved successfully",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Suggest crate dependencies based on code patterns")]
    async fn suggest_dependencies(
        &self,
        Parameters(SuggestDependenciesParams {
            query,
            workspace_path,
        }): Parameters<SuggestDependenciesParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "query": query,
            "workspace_path": workspace_path
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("suggest_dependencies", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "Dependencies suggested successfully",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Create a new Rust module with optional visibility")]
    async fn create_module(
        &self,
        Parameters(CreateModuleParams {
            module_name,
            module_path,
            is_public,
        }): Parameters<CreateModuleParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "module_name": module_name,
            "module_path": module_path,
            "is_public": is_public
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("create_module", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "Module created successfully",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }

    #[tool(description = "Move code items from one file to another")]
    async fn move_items(
        &self,
        Parameters(MoveItemsParams {
            source_file,
            target_file,
            item_names,
        }): Parameters<MoveItemsParams>,
    ) -> Result<CallToolResult, McpError> {
        let args = serde_json::json!({
            "source_file": source_file,
            "target_file": target_file,
            "item_names": item_names
        });

        let mut analyzer = self.analyzer.lock().await;
        match execute_tool("move_items", args, &mut analyzer).await {
            Ok(result) => {
                if let Some(content) = result.content.first() {
                    if let Some(text) = content.get("text") {
                        return Ok(CallToolResult::success(vec![Content::text(
                            text.as_str().unwrap_or("No result"),
                        )]));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(
                    "Items moved successfully",
                )]))
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }
}

#[tool_handler]
impl ServerHandler for RustMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("Rust MCP Server providing rust-analyzer integration for idiomatic Rust development tools. Provides code analysis, refactoring, and project management capabilities.".to_string()),
        }
    }
}
