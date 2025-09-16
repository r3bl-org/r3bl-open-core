# R3BL TUI Documentation

Welcome to the R3BL TUI library documentation! This directory contains comprehensive guides, architectural documentation, and technical references for the terminal user interface framework.

## Documentation Index

### Core Guides

- **[ANSI Testing Guide](ANSI_TESTING.md)** - Comprehensive guide to VT100 ANSI conformance testing
  - Type-safe sequence builders vs hardcoded escape strings
  - Real-world testing scenarios (vim, emacs, tmux patterns)
  - Running and debugging conformance tests
  - Adding new VT100 compliance tests

### Architecture Documentation

The main architectural documentation is embedded in the Rust module documentation. Use `cargo doc --no-deps --open` to generate and view the complete API documentation, or explore the key modules:

- **`src/lib.rs`** - Main library overview and feature documentation
- **`src/core/pty_mux/ansi_parser/mod.rs`** - ANSI parser architecture and VT100 compliance
- **`src/core/pty_mux/ansi_parser/perform.rs`** - Detailed VTE parser integration
- **`src/core/pty_mux/ansi_parser/vt_100_ansi_conformance_tests/mod.rs`** - Testing infrastructure

### Quick Reference

#### Development Commands

```bash
# Run all tests
cargo test

# Run VT100 ANSI conformance tests
cargo test vt_100_ansi_conformance_tests

# Run specific test categories
cargo test test_real_world_scenarios
cargo test test_cursor_operations
cargo test test_sgr_and_character_sets

# Generate documentation
cargo doc --no-deps --open

# Development workflow
cargo watch -x "test vt_100_ansi_conformance_tests"
```

#### Testing Categories

| Category | Purpose | Command |
|----------|---------|---------|
| **VT100 Conformance** | ANSI sequence compliance | `cargo test vt_100_ansi_conformance_tests` |
| **Real-world Scenarios** | Application patterns | `cargo test test_real_world_scenarios` |
| **Integration Tests** | Full pipeline testing | `cargo test ansi_integration_tests` |
| **Unit Tests** | Component-level testing | `cargo test --lib` |

#### Key Testing Features

- **101+ conformance tests** validating VT100/ANSI specification compliance
- **Type-safe sequence builders** for compile-time validation
- **Realistic terminal dimensions** (80x25) for authentic testing
- **Real application patterns** from vim, emacs, tmux, and other terminal apps
- **Comprehensive edge case coverage** including malformed sequences and boundary conditions

## Contributing

### Adding Documentation

1. **Module documentation**: Add comprehensive `//!` comments to module files
2. **Architectural guides**: Create detailed markdown files in this `docs/` directory
3. **Testing guides**: Document testing patterns and add examples
4. **API documentation**: Use `///` comments for public functions and types

### Documentation Standards

- Use clear, concise language with technical accuracy
- Include code examples for complex concepts
- Reference VT100/ANSI specifications where applicable
- Maintain consistency with existing documentation style
- Test all code examples to ensure they compile and work correctly

### Regenerating README Files

This project uses `cargo readme` to generate README files from module documentation:

```bash
# Generate README for main library
cargo readme --template README.tpl.md > README.md

# Generate README for ANSI parser module
cd src/core/pty_mux/ansi_parser
cargo readme > README.md

# Generate README for conformance tests
cd vt_100_ansi_conformance_tests
cargo readme > README.md
```

## External Resources

### VT100/ANSI Specifications

- **[VT100 User Guide](https://vt100.net/docs/vt100-ug/)** - Official VT100 terminal documentation
- **[ANSI X3.64 Standard](https://www.ecma-international.org/wp-content/uploads/ECMA-48_5th_edition_june_1991.pdf)** - ECMA-48 control functions specification
- **[XTerm Control Sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html)** - Extended terminal sequence reference

### Terminal Testing Resources

- **[VTE Library](https://gitlab.gnome.org/GNOME/vte)** - GNOME terminal widget (same parser we use)
- **[Alacritty](https://github.com/alacritty/alacritty)** - Reference terminal implementation
- **[Terminal.app Tests](https://github.com/alacritty/alacritty/tree/master/alacritty_terminal/src/ansi)** - Real-world test patterns

### Rust TUI Ecosystem

- **[Crossterm](https://github.com/crossterm-rs/crossterm)** - Cross-platform terminal manipulation
- **[Ratatui](https://github.com/ratatui-org/ratatui)** - Alternative Rust TUI framework
- **[Cursive](https://github.com/gyscos/cursive)** - High-level TUI library

---

For questions about the documentation or suggestions for improvements, please open an issue in the [r3bl-open-core repository](https://github.com/r3bl-org/r3bl-open-core).