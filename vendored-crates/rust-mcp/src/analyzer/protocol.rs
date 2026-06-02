use serde_json::{Value, json};

pub fn create_text_document_position_params(file_path: &str, line: u32, character: u32) -> Value {
    json!({
        "textDocument": {
            "uri": format!("file://{}", file_path)
        },
        "position": {
            "line": line,
            "character": character
        }
    })
}

pub fn create_references_params(file_path: &str, line: u32, character: u32) -> Value {
    json!({
        "textDocument": {
            "uri": format!("file://{}", file_path)
        },
        "position": {
            "line": line,
            "character": character
        },
        "context": {
            "includeDeclaration": true
        }
    })
}

pub fn create_workspace_symbol_params(query: &str) -> Value {
    json!({
        "query": query
    })
}

pub fn create_rename_params(file_path: &str, line: u32, character: u32, new_name: &str) -> Value {
    json!({
        "textDocument": {
            "uri": format!("file://{}", file_path)
        },
        "position": {
            "line": line,
            "character": character
        },
        "newName": new_name
    })
}

pub fn create_formatting_params(file_path: &str) -> Value {
    json!({
        "textDocument": {
            "uri": format!("file://{}", file_path)
        },
        "options": {
            "tabSize": 4,
            "insertSpaces": true
        }
    })
}
