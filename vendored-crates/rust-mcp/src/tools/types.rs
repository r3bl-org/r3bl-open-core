use anyhow::Result;
use serde_json::{Value, json};
use std::borrow::Cow;
use std::sync::Arc;

use crate::analyzer::RustAnalyzerClient;

pub struct ToolDefinition {
    pub name: Cow<'static, str>,
    pub description: Cow<'static, str>,
    pub input_schema: Arc<serde_json::Map<String, Value>>,
}

impl ToolDefinition {
    pub fn new(name: &'static str, description: &'static str, schema: Value) -> Self {
        let schema_map = match schema {
            Value::Object(map) => Arc::new(map),
            _ => Arc::new(serde_json::Map::new()),
        };

        Self {
            name: Cow::Borrowed(name),
            description: Cow::Borrowed(description),
            input_schema: schema_map,
        }
    }
}

pub struct ToolResult {
    pub content: Vec<serde_json::Map<String, Value>>,
}

pub async fn execute_tool(
    name: &str,
    args: Value,
    analyzer: &mut RustAnalyzerClient,
) -> Result<ToolResult> {
    match name {
        "find_definition" => crate::tools::analysis::find_definition_impl(args, analyzer).await,
        "find_references" => crate::tools::analysis::find_references_impl(args, analyzer).await,
        "get_diagnostics" => crate::tools::analysis::get_diagnostics_impl(args, analyzer).await,
        "workspace_symbols" => {
            crate::tools::navigation::workspace_symbols_impl(args, analyzer).await
        }
        "rename_symbol" => crate::tools::refactoring::rename_symbol_impl(args, analyzer).await,
        "extract_function" => {
            crate::tools::refactoring::extract_function_impl(args, analyzer).await
        }
        "format_code" => crate::tools::formatting::format_code_impl(args, analyzer).await,
        "analyze_manifest" => crate::tools::cargo::analyze_manifest_impl(args, analyzer).await,
        "run_cargo_check" => crate::tools::cargo::run_cargo_check_impl(args, analyzer).await,
        "generate_struct" => crate::tools::generation::generate_struct_impl(args, analyzer).await,
        "generate_enum" => crate::tools::generation::generate_enum_impl(args, analyzer).await,
        "generate_trait_impl" => {
            crate::tools::generation::generate_trait_impl_impl(args, analyzer).await
        }
        "generate_tests" => crate::tools::generation::generate_tests_impl(args, analyzer).await,
        "inline_function" => crate::tools::refactoring::inline_function_impl(args, analyzer).await,
        "change_signature" => {
            crate::tools::refactoring::change_signature_impl(args, analyzer).await
        }
        "organize_imports" => {
            crate::tools::refactoring::organize_imports_impl(args, analyzer).await
        }
        "apply_clippy_suggestions" => {
            crate::tools::quality::apply_clippy_suggestions_impl(args, analyzer).await
        }
        "validate_lifetimes" => {
            crate::tools::quality::validate_lifetimes_impl(args, analyzer).await
        }
        "get_type_hierarchy" => {
            crate::tools::advanced::get_type_hierarchy_impl(args, analyzer).await
        }
        "suggest_dependencies" => {
            crate::tools::advanced::suggest_dependencies_impl(args, analyzer).await
        }
        "create_module" => crate::tools::advanced::create_module_impl(args, analyzer).await,
        "move_items" => crate::tools::advanced::move_items_impl(args, analyzer).await,
        _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
    }
}

pub fn get_tools() -> Vec<ToolDefinition> {
    vec![
        // Code Analysis
        ToolDefinition::new(
            "find_definition",
            "Find the definition of a symbol at a given position",
            json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string"},
                    "line": {"type": "number"},
                    "character": {"type": "number"}
                },
                "required": ["file_path", "line", "character"]
            }),
        ),
        ToolDefinition::new(
            "find_references",
            "Find all references to a symbol at a given position",
            json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string"},
                    "line": {"type": "number"},
                    "character": {"type": "number"}
                },
                "required": ["file_path", "line", "character"]
            }),
        ),
        ToolDefinition::new(
            "get_diagnostics",
            "Get compiler diagnostics for a file",
            json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string"}
                },
                "required": ["file_path"]
            }),
        ),
        ToolDefinition::new(
            "workspace_symbols",
            "Search for symbols in the workspace",
            json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"}
                },
                "required": ["query"]
            }),
        ),
        ToolDefinition::new(
            "rename_symbol",
            "Rename a symbol with scope awareness",
            json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string"},
                    "line": {"type": "number"},
                    "character": {"type": "number"},
                    "new_name": {"type": "string"}
                },
                "required": ["file_path", "line", "character", "new_name"]
            }),
        ),
        ToolDefinition::new(
            "format_code",
            "Apply rustfmt formatting to a file",
            json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string"}
                },
                "required": ["file_path"]
            }),
        ),
        ToolDefinition::new(
            "analyze_manifest",
            "Parse and analyze Cargo.toml file",
            json!({
                "type": "object",
                "properties": {
                    "manifest_path": {"type": "string"}
                },
                "required": ["manifest_path"]
            }),
        ),
        ToolDefinition::new(
            "run_cargo_check",
            "Execute cargo check and parse errors",
            json!({
                "type": "object",
                "properties": {
                    "workspace_path": {"type": "string"}
                },
                "required": ["workspace_path"]
            }),
        ),
        ToolDefinition::new(
            "extract_function",
            "Extract selected code into a new function",
            json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string"},
                    "start_line": {"type": "number"},
                    "start_character": {"type": "number"},
                    "end_line": {"type": "number"},
                    "end_character": {"type": "number"},
                    "function_name": {"type": "string"}
                },
                "required": ["file_path", "start_line", "start_character", "end_line", "end_character", "function_name"]
            }),
        ),
        ToolDefinition::new(
            "generate_struct",
            "Generate a struct with specified fields and derives",
            json!({
                "type": "object",
                "properties": {
                    "struct_name": {"type": "string"},
                    "fields": {"type": "array", "items": {"type": "object"}},
                    "derives": {"type": "array", "items": {"type": "string"}},
                    "file_path": {"type": "string"}
                },
                "required": ["struct_name", "fields", "file_path"]
            }),
        ),
        ToolDefinition::new(
            "generate_enum",
            "Generate an enum with specified variants and derives",
            json!({
                "type": "object",
                "properties": {
                    "enum_name": {"type": "string"},
                    "variants": {"type": "array", "items": {"type": "object"}},
                    "derives": {"type": "array", "items": {"type": "string"}},
                    "file_path": {"type": "string"}
                },
                "required": ["enum_name", "variants", "file_path"]
            }),
        ),
        ToolDefinition::new(
            "generate_trait_impl",
            "Generate a trait implementation for a struct",
            json!({
                "type": "object",
                "properties": {
                    "trait_name": {"type": "string"},
                    "struct_name": {"type": "string"},
                    "file_path": {"type": "string"}
                },
                "required": ["trait_name", "struct_name", "file_path"]
            }),
        ),
        ToolDefinition::new(
            "generate_tests",
            "Generate unit tests for a function",
            json!({
                "type": "object",
                "properties": {
                    "target_function": {"type": "string"},
                    "file_path": {"type": "string"},
                    "test_cases": {"type": "array", "items": {"type": "object"}}
                },
                "required": ["target_function", "file_path"]
            }),
        ),
        ToolDefinition::new(
            "inline_function",
            "Inline a function call at specified position",
            json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string"},
                    "line": {"type": "number"},
                    "character": {"type": "number"}
                },
                "required": ["file_path", "line", "character"]
            }),
        ),
        ToolDefinition::new(
            "change_signature",
            "Change the signature of a function",
            json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string"},
                    "line": {"type": "number"},
                    "character": {"type": "number"},
                    "new_signature": {"type": "string"}
                },
                "required": ["file_path", "line", "character", "new_signature"]
            }),
        ),
        ToolDefinition::new(
            "organize_imports",
            "Organize and sort import statements in a file",
            json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string"}
                },
                "required": ["file_path"]
            }),
        ),
        ToolDefinition::new(
            "apply_clippy_suggestions",
            "Apply clippy lint suggestions to improve code quality",
            json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string"}
                },
                "required": ["file_path"]
            }),
        ),
        ToolDefinition::new(
            "validate_lifetimes",
            "Validate and suggest lifetime annotations",
            json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string"}
                },
                "required": ["file_path"]
            }),
        ),
        ToolDefinition::new(
            "get_type_hierarchy",
            "Get type hierarchy for a symbol at specified position",
            json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string"},
                    "line": {"type": "integer", "minimum": 0},
                    "character": {"type": "integer", "minimum": 0}
                },
                "required": ["file_path", "line", "character"]
            }),
        ),
        ToolDefinition::new(
            "suggest_dependencies",
            "Suggest crate dependencies based on code patterns",
            json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "workspace_path": {"type": "string"}
                },
                "required": ["query", "workspace_path"]
            }),
        ),
        ToolDefinition::new(
            "create_module",
            "Create a new Rust module with optional visibility",
            json!({
                "type": "object",
                "properties": {
                    "module_name": {"type": "string"},
                    "module_path": {"type": "string"},
                    "is_public": {"type": "boolean"}
                },
                "required": ["module_name", "module_path"]
            }),
        ),
        ToolDefinition::new(
            "move_items",
            "Move code items from one file to another",
            json!({
                "type": "object",
                "properties": {
                    "source_file": {"type": "string"},
                    "target_file": {"type": "string"},
                    "item_names": {
                        "type": "array",
                        "items": {"type": "string"}
                    }
                },
                "required": ["source_file", "target_file", "item_names"]
            }),
        ),
    ]
}
