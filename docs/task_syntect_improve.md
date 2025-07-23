<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Syntect Improvement Plan: Adding Missing Language Support](#syntect-improvement-plan-adding-missing-language-support)
  - [Overview](#overview)
  - [Current Situation](#current-situation)
  - [Primary Approach: Add Custom .sublime-syntax Files](#primary-approach-add-custom-sublime-syntax-files)
    - [Implementation Steps](#implementation-steps)
  - [Backup Approach: Alternative Crates](#backup-approach-alternative-crates)
  - [Success Criteria](#success-criteria)
  - [Next Steps](#next-steps)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Syntect Improvement Plan: Adding Missing Language Support

## Overview

This document outlines the plan to add support for TypeScript, TOML, SCSS, Kotlin, Swift, and
Dockerfile languages in syntect by adding custom `.sublime-syntax` files.

## Current Situation

The syntect crate currently does not support the following languages:

- TypeScript
- TOML
- SCSS
- Kotlin
- Swift
- Dockerfile

## Primary Approach: Add Custom .sublime-syntax Files

We will first attempt to add missing language support by creating/adding custom `.sublime-syntax`
files for each unsupported language. This approach is preferred because:

1. It leverages syntect's existing infrastructure
2. Minimal code changes required
3. Maintains consistency with current implementation
4. syntect provides built-in support for loading additional syntax definitions

### Implementation Steps

1. **Create syntax directory structure**
   - Create `syntaxes/` directory in the project
   - Organize by language: `syntaxes/typescript/`, `syntaxes/toml/`, etc.

2. **Obtain .sublime-syntax files**
   - Source from official Sublime Text packages
   - Or from community-maintained syntax definitions
   - Ensure licensing compatibility

3. **Integrate with syntect**

   ```rust
   pub fn load_syntax_set() -> Result<SyntaxSet> {
       let mut builder = SyntaxSet::load_defaults_newlines().into_builder();
       builder.add_from_folder("path/to/syntaxes", true)?;
       let syntax_set = builder.build();
       Ok(syntax_set)
   }
   ```

4. **Test each language**
   - Create test files for each language
   - Verify syntax highlighting works correctly
   - Check performance impact

5. **Update file extension mappings**
   - Ensure proper file extensions are mapped to syntax definitions
   - Handle edge cases (e.g., .ts vs .tsx for TypeScript)

## Backup Approach: Alternative Crates

If adding custom .sublime-syntax files proves unsuccessful or inadequate, we will consider migrating
to alternative syntax highlighting crates:

1. **synoptic** - Modern syntax highlighting library
2. **inkjet** - GPU-accelerated syntax highlighting
3. **tree-sitter-highlight** - Tree-sitter based highlighting
4. **shiki** - TextMate grammar based highlighting

This evaluation will only be pursued if the primary approach fails to deliver satisfactory results.

## Success Criteria

- All six languages (TypeScript, TOML, SCSS, Kotlin, Swift, Dockerfile) have working syntax
  highlighting
- Performance remains acceptable (no significant slowdown)
- Integration is maintainable and doesn't require extensive code changes

## Next Steps

1. Research and locate appropriate .sublime-syntax files for each language
2. Create proof-of-concept with one language (suggest starting with TOML as it's simpler)
3. If successful, extend to remaining languages
4. Document the process for future language additions
