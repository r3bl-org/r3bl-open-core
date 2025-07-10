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

//! Jumbo-sized markdown inputs for performance testing.
//!
//! These inputs will contain real-world large markdown files:
//! - Blog posts and articles from developerlife.com
//! - Large README files from open source projects
//! - Technical documentation and guides
//!
//! Note: Real content will be added in Phase 2 using `include_str!` macro

// Placeholder for real-world content (Phase 2)
pub const REAL_WORLD_EDITOR_CONTENT: &str = r#"# Real World Content Placeholder

This content will be replaced in Phase 2 with the actual `get_real_world_editor_content()`
function content, which appears to be a substantial piece of markdown content used for
testing the editor component.

The actual content should be moved here from the function and accessed via include_str!
for better performance and organization.

For now, this serves as a placeholder to maintain the module structure.
"#;

// Placeholder for large story content (Phase 2)
#[allow(dead_code)]
pub const LARGE_STORY_CONTENT: &str = r#"# Large Story Content Placeholder

This will contain the content from the story file:
2015-11-08-struggle-meet-greatness.md

This should provide a substantial real-world markdown document for performance testing
of both legacy and NG parsers.
"#;

// Additional placeholders for Phase 2 real-world content
#[allow(dead_code)]
pub const TECHNICAL_DOCUMENTATION: &str = r#"# Technical Documentation Placeholder

This will contain large technical documentation files, possibly:
- README files from major open source projects
- API documentation
- Tutorial content
- Technical blog posts

These will provide realistic performance testing scenarios.
"#;
