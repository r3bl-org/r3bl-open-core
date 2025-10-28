# Syntect Improvement Plan: Adding Missing Language Support

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Overview](#overview)
  - [Task Description](#task-description)
  - [Current State](#current-state)
  - [Goals](#goals)
- [Implementation plan](#implementation-plan)
  - [Step 0: Add Custom .sublime-syntax Files [PENDING]](#step-0-add-custom-sublime-syntax-files-pending)
    - [Approach Overview](#approach-overview)
    - [Implementation Steps](#implementation-steps)
    - [Detailed Tasks](#detailed-tasks)
  - [Step 1: Backup Approach - Alternative Crates [PENDING]](#step-1-backup-approach---alternative-crates-pending)
  - [Success Criteria](#success-criteria)
    - [Must Have](#must-have)
    - [Should Have](#should-have)
    - [Nice to Have](#nice-to-have)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Overview

## Task Description

Add support for TypeScript, TOML, SCSS, Kotlin, Swift, and Dockerfile languages in syntect by adding
custom `.sublime-syntax` files. This expands the syntax highlighting capabilities of the codebase to
support a broader range of programming languages and file formats.

## Current State

The syntect crate currently does not support the following languages:

- TypeScript
- TOML
- SCSS
- Kotlin
- Swift
- Dockerfile

Without these languages, developers working with these file types won't have proper syntax
highlighting in the TUI applications.

## Goals

1. Add custom `.sublime-syntax` files for all six unsupported languages
2. Integrate language definitions with syntect's existing infrastructure
3. Ensure syntax highlighting works correctly for each language
4. Maintain acceptable performance with no significant slowdown
5. Create a maintainable solution for future language additions

# Implementation plan

## Step 0: Add Custom .sublime-syntax Files [PENDING]

### Approach Overview

Add missing language support by creating/adding custom `.sublime-syntax` files for each unsupported
language. This approach is preferred because:

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

### Detailed Tasks

- [ ] Research and locate appropriate .sublime-syntax files for each language
- [ ] Create proof-of-concept with TOML (simpler language to start)
- [ ] Implement directory structure for syntaxes
- [ ] Integrate with syntect using builder pattern
- [ ] Create test files and verify highlighting
- [ ] Extend to remaining languages (TypeScript, SCSS, Kotlin, Swift, Dockerfile)
- [ ] Document the process for future language additions

## Step 1: Backup Approach - Alternative Crates [PENDING]

If adding custom .sublime-syntax files proves unsuccessful or inadequate, we will consider migrating
to alternative syntax highlighting crates:

1. **synoptic** - Modern syntax highlighting library
2. **inkjet** - GPU-accelerated syntax highlighting
3. **tree-sitter-highlight** - Tree-sitter based highlighting
4. **shiki** - TextMate grammar based highlighting

This evaluation will only be pursued if the primary approach fails to deliver satisfactory results.

## Success Criteria

### Must Have

- All six languages (TypeScript, TOML, SCSS, Kotlin, Swift, Dockerfile) have working syntax
  highlighting
- Performance remains acceptable (no significant slowdown compared to current implementation)

### Should Have

- Integration is maintainable and doesn't require extensive code changes
- Process is documented for future language additions

### Nice to Have

- Support for additional common languages
- Customizable color schemes per language
