/*
 *   Copyright (c) 2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

//! Large-scale markdown inputs for comprehensive testing.
//!
//! These inputs represent larger documents with complex structures and mixed content:
//! - Long multi-paragraph documents
//! - Complex nested structures
//! - Large amounts of content approximating real-world usage

// Complex document structure
pub const BLOG_POST_DOCUMENT: &str = r#"# A Complete Guide to Markdown

## Introduction

Markdown is a lightweight markup language with plain text formatting syntax. Its design allows it to be converted to many output formats, but the original tool by the same name only supports HTML.

Markdown is often used to format readme files, for writing messages in online discussion forums, and to create rich text using a plain text editor.

## Basic Syntax

### Headers

You can create headers using the `#` symbol:

```markdown
# H1 Header
## H2 Header  
### H3 Header
#### H4 Header
##### H5 Header
###### H6 Header
```

### Emphasis

You can make text *italic* or **bold**:

- *This text is italic*
- **This text is bold** 
- ***This text is both italic and bold***

### Lists

Unordered lists:
- Item 1
- Item 2
  - Nested item 2.1
  - Nested item 2.2
- Item 3

Ordered lists:
1. First item
2. Second item
   1. Nested item 2.1
   2. Nested item 2.2
3. Third item

### Code

Inline code: `console.log("Hello World")`

Code blocks:
```javascript
function greet(name) {
    console.log(`Hello, ${name}!`);
}
greet("World");
```

## Advanced Features

### Tables

| Column 1 | Column 2 | Column 3 |
|----------|----------|----------|
| Row 1    | Data     | More     |
| Row 2    | Data     | More     |

### Task Lists

- [x] Write the press release
- [ ] Update the website
- [ ] Contact the media

## Conclusion

Markdown provides a simple way to format text that's both human-readable and machine-processable. Its simplicity and versatility make it an excellent choice for documentation, blogging, and note-taking.

---

*This guide covers the essential Markdown syntax. For more advanced features, consult the official documentation.*
"#;

// Large nested structure
pub const COMPLEX_NESTED_DOCUMENT: &str = r#"# Project Documentation

## Overview
This is a comprehensive overview of our project structure and implementation details.

### Architecture
Our system follows a modular architecture with the following components:

#### Core Module
- **Parser**: Handles markdown parsing
  - Legacy parser (backward compatibility)
  - NG parser (new generation)
  - Performance optimizations
- **Renderer**: Converts parsed content to output
  - HTML renderer
  - Terminal renderer
  - Custom formatters

#### Utilities Module
- Configuration management
- Logging and debugging
- Error handling
- Performance monitoring

### Implementation Details

#### Parser Implementation

The parser consists of several phases:

1. **Lexical Analysis**
   ```rust
   fn tokenize(input: &str) -> Vec<Token> {
       // Implementation details
   }
   ```

2. **Syntax Analysis**
   ```rust
   fn parse(tokens: Vec<Token>) -> ParseResult {
       // Implementation details
   }
   ```

3. **Semantic Analysis**
   ```rust
   fn analyze(ast: AST) -> AnalysisResult {
       // Implementation details
   }
   ```

#### Performance Considerations

Our benchmarks show:
- Legacy parser: ~100ms for large documents
- NG parser: ~50ms for large documents (2x improvement)
- Memory usage: 30% reduction with NG parser

### Testing Strategy

We employ multiple testing approaches:

#### Unit Tests
- Individual function testing
- Edge case validation
- Error condition handling

#### Integration Tests
- End-to-end workflow testing
- Component interaction validation
- Performance regression testing

#### Compatibility Tests
- Legacy vs NG parser comparison
- Output format validation
- Backward compatibility verification

### Future Roadmap

1. **Phase 1**: Complete NG parser implementation
2. **Phase 2**: Performance optimizations
3. **Phase 3**: Additional output formats

## Conclusion

This documentation provides a comprehensive overview of our markdown parsing system. Regular updates ensure accuracy and completeness.
"#;

// Real-world content simulation
pub const TUTORIAL_DOCUMENT: &str = r#"# Getting Started with Rust Terminal Applications

## Prerequisites

Before we begin, make sure you have:

- Rust installed (1.70.0 or later)
- A terminal or command prompt
- Your favorite text editor or IDE

## Setting Up Your Project

### 1. Create a New Rust Project

```bash
cargo new my_terminal_app
cd my_terminal_app
```

### 2. Add Dependencies

Edit your `Cargo.toml` file:

```toml
[dependencies]
clap = "4.0"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
```

### 3. Project Structure

Your project should look like this:

```
my_terminal_app/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── lib.rs
│   └── modules/
│       ├── cli.rs
│       ├── config.rs
│       └── commands/
│           ├── mod.rs
│           ├── init.rs
│           └── run.rs
└── README.md
```

## Building Your First Command

### Command Line Interface

Create a basic CLI structure:

```rust
use clap::{Arg, Command};

fn main() {
    let matches = Command::new("my_terminal_app")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("A sample terminal application")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(clap::ArgAction::Count)
                .help("Sets the level of verbosity")
        )
        .subcommand(
            Command::new("init")
                .about("Initialize a new project")
                .arg(
                    Arg::new("name")
                        .help("The name of the project")
                        .required(true)
                        .index(1)
                )
        )
        .get_matches();

    // Handle matches
    match matches.subcommand() {
        Some(("init", sub_matches)) => {
            let name = sub_matches.get_one::<String>("name").unwrap();
            println!("Initializing project: {}", name);
        }
        _ => {
            println!("No subcommand was used");
        }
    }
}
```

### Configuration Management

Create a configuration system:

```rust
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub app_name: String,
    pub version: String,
    pub debug: bool,
    pub output_dir: String,
}

impl Config {
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let toml_string = toml::to_string(self)?;
        fs::write(path, toml_string)?;
        Ok(())
    }
}
```

## Advanced Features

### Async Operations

For handling async operations:

```rust
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Your async code here
    let result = fetch_data().await?;
    println!("Fetched: {}", result);
    Ok(())
}

async fn fetch_data() -> Result<String, Box<dyn std::error::Error>> {
    // Simulate async operation
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    Ok("Sample data".to_string())
}
```

### Error Handling

Implement robust error handling:

```rust
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    ConfigError(String),
    NetworkError(String),
    FileError(std::io::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            AppError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            AppError::FileError(err) => write!(f, "File error: {}", err),
        }
    }
}

impl std::error::Error for AppError {}
```

## Testing Your Application

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = Config {
            app_name: "test_app".to_string(),
            version: "1.0.0".to_string(),
            debug: true,
            output_dir: "/tmp".to_string(),
        };
        
        assert_eq!(config.app_name, "test_app");
        assert!(config.debug);
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = fetch_data().await;
        assert!(result.is_ok());
    }
}
```

### Integration Tests

Create integration tests in `tests/` directory:

```rust
// tests/integration_test.rs
use my_terminal_app::Config;

#[test]
fn test_full_workflow() {
    // Test complete application workflow
    let config = Config::load_from_file("test_config.toml").unwrap();
    // ... rest of the test
}
```

## Deployment and Distribution

### Building for Release

```bash
cargo build --release
```

### Cross-platform Compilation

```bash
# For Linux
cargo build --target x86_64-unknown-linux-gnu --release

# For Windows
cargo build --target x86_64-pc-windows-gnu --release

# For macOS
cargo build --target x86_64-apple-darwin --release
```

## Best Practices

1. **Code Organization**
   - Use modules to organize code
   - Separate concerns into different files
   - Follow Rust naming conventions

2. **Error Handling**
   - Use `Result<T, E>` for fallible operations
   - Create custom error types
   - Provide meaningful error messages

3. **Testing**
   - Write unit tests for individual functions
   - Create integration tests for workflows
   - Use property-based testing for complex logic

4. **Performance**
   - Profile your application regularly
   - Use appropriate data structures
   - Consider async/await for I/O operations

5. **Documentation**
   - Write clear comments and documentation
   - Provide usage examples
   - Keep README updated

## Conclusion

You now have a solid foundation for building terminal applications in Rust. This tutorial covered the essential concepts and provided practical examples to get you started.

For more advanced topics, consider exploring:
- Terminal UI libraries (tui-rs, crossterm)
- Advanced async patterns
- Plugin architectures
- Configuration management systems

Happy coding! 🦀
"#;
