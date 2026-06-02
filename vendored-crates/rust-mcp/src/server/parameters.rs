use rmcp::schemars;

// Parameter structs for tools
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FindDefinitionParams {
    pub file_path: String,
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FindReferencesParams {
    pub file_path: String,
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetDiagnosticsParams {
    pub file_path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WorkspaceSymbolsParams {
    pub query: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RenameSymbolParams {
    pub file_path: String,
    pub line: u32,
    pub character: u32,
    pub new_name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FormatCodeParams {
    pub file_path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AnalyzeManifestParams {
    pub manifest_path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RunCargoCheckParams {
    pub workspace_path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExtractFunctionParams {
    pub file_path: String,
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
    pub function_name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GenerateStructParams {
    pub struct_name: String,
    pub fields: Vec<serde_json::Value>,
    pub derives: Option<Vec<String>>,
    pub file_path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GenerateEnumParams {
    pub enum_name: String,
    pub variants: Vec<serde_json::Value>,
    pub derives: Option<Vec<String>>,
    pub file_path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GenerateTraitImplParams {
    pub trait_name: String,
    pub struct_name: String,
    pub file_path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GenerateTestsParams {
    pub target_function: String,
    pub file_path: String,
    pub test_cases: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InlineFunctionParams {
    pub file_path: String,
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ChangeSignatureParams {
    pub file_path: String,
    pub line: u32,
    pub character: u32,
    pub new_signature: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct OrganizeImportsParams {
    pub file_path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ApplyClippySuggestionsParams {
    pub file_path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ValidateLifetimesParams {
    pub file_path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetTypeHierarchyParams {
    pub file_path: String,
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SuggestDependenciesParams {
    pub query: String,
    pub workspace_path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateModuleParams {
    pub module_name: String,
    pub module_path: String,
    pub is_public: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MoveItemsParams {
    pub source_file: String,
    pub target_file: String,
    pub item_names: Vec<String>,
}
