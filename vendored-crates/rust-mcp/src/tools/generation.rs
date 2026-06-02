use crate::analyzer::RustAnalyzerClient;
use crate::tools::types::ToolResult;
use anyhow::Result;
use serde_json::{Value, json};

pub async fn generate_struct_impl(
    args: Value,
    analyzer: &mut RustAnalyzerClient,
) -> Result<ToolResult> {
    let struct_name = args
        .get("struct_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing struct_name parameter"))?;
    let fields = args
        .get("fields")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Missing fields parameter"))?;
    let derives = args
        .get("derives")
        .and_then(|v| v.as_array())
        .map(|v| v.iter().filter_map(|d| d.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();
    let file_path = args
        .get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing file_path parameter"))?;

    let result = analyzer
        .generate_struct(struct_name, fields, &derives, file_path)
        .await?;

    Ok(ToolResult {
        content: vec![
            json!({
                "type": "text",
                "text": result
            })
            .as_object()
            .unwrap()
            .clone(),
        ],
    })
}

pub async fn generate_enum_impl(
    args: Value,
    analyzer: &mut RustAnalyzerClient,
) -> Result<ToolResult> {
    let enum_name = args
        .get("enum_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing enum_name parameter"))?;
    let variants = args
        .get("variants")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Missing variants parameter"))?;
    let derives = args
        .get("derives")
        .and_then(|v| v.as_array())
        .map(|v| v.iter().filter_map(|d| d.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();
    let file_path = args
        .get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing file_path parameter"))?;

    let result = analyzer
        .generate_enum(enum_name, variants, &derives, file_path)
        .await?;

    Ok(ToolResult {
        content: vec![
            json!({
                "type": "text",
                "text": result
            })
            .as_object()
            .unwrap()
            .clone(),
        ],
    })
}

pub async fn generate_trait_impl_impl(
    args: Value,
    analyzer: &mut RustAnalyzerClient,
) -> Result<ToolResult> {
    let trait_name = args
        .get("trait_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing trait_name parameter"))?;
    let struct_name = args
        .get("struct_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing struct_name parameter"))?;
    let file_path = args
        .get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing file_path parameter"))?;

    let result = analyzer
        .generate_trait_impl(trait_name, struct_name, file_path)
        .await?;

    Ok(ToolResult {
        content: vec![
            json!({
                "type": "text",
                "text": result
            })
            .as_object()
            .unwrap()
            .clone(),
        ],
    })
}

pub async fn generate_tests_impl(
    args: Value,
    analyzer: &mut RustAnalyzerClient,
) -> Result<ToolResult> {
    let target_function = args
        .get("target_function")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing target_function parameter"))?;
    let file_path = args
        .get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing file_path parameter"))?;
    let empty_cases = vec![];
    let test_cases = args
        .get("test_cases")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty_cases);

    let result = analyzer
        .generate_tests(target_function, file_path, test_cases)
        .await?;

    Ok(ToolResult {
        content: vec![
            json!({
                "type": "text",
                "text": result
            })
            .as_object()
            .unwrap()
            .clone(),
        ],
    })
}
