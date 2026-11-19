// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Reference definition collection, target reconciliation, alphabetical sorting, and
//! CST layout management for technical terms at the bottom of doc comment blocks.

use super::TechnicalTermDictionary;
use crate::cargo_rustdoc_fmt::{constants::{BACKTICK_CHAR, BACKTICKED_LINK_PREFIX,
                                           RUST_PATH_SEPARATOR},
                               cst::{DocBlockCst, DocNode, LinkReference,
                                     split_reference_def_label_and_rest}};
use r3bl_tui::HashMap;
use std::collections::BTreeSet;

/// Pre-existing reference definitions extracted and indexed from a [`DocBlockCst`].
#[derive(Debug, Clone)]
pub(super) struct ExistingRefDefs {
    pub(super) refs: Vec<LinkReference>,
    pub(super) ref_node_count: usize,
    pub(super) ref_def_map: HashMap<String, (String, LinkReference)>,
}

/// Collects existing reference definitions from all [`DocNode::ReferenceDefinitions`]
/// nodes in `nodes` and indexes them by term key.
pub(super) fn collect_existing_ref_defs(nodes: &[DocNode]) -> ExistingRefDefs {
    let mut existing_refs: Vec<LinkReference> = Vec::new();
    let mut ref_node_count = 0;
    for node in nodes {
        if let DocNode::ReferenceDefinitions { definitions, .. } = node {
            ref_node_count += 1;
            existing_refs.extend(definitions.iter().cloned());
        }
    }

    let mut ref_def_map: HashMap<String, (String, LinkReference)> = HashMap::default();
    for def in &existing_refs {
        let rendered = def.render();
        if let Some((term, target)) = parse_ref_def(&rendered) {
            let is_backticked = rendered.trim_start().starts_with(BACKTICKED_LINK_PREFIX);
            if let Some((existing_target, existing_def)) = ref_def_map.get(&term)
                && existing_def
                    .render()
                    .trim_start()
                    .starts_with(BACKTICKED_LINK_PREFIX)
                && (!is_backticked || existing_target.contains(RUST_PATH_SEPARATOR))
            {
                continue;
            }
            ref_def_map.insert(term, (target, def.clone()));
        }
    }

    ExistingRefDefs {
        refs: existing_refs,
        ref_node_count,
        ref_def_map,
    }
}

/// Reconciles linked terms against canonical dictionary targets and pre-existing
/// reference definitions, sorts them alphabetically (case-insensitive), and
/// deduplicates.
pub(super) fn build_final_ref_defs(
    linked_terms: &BTreeSet<String>,
    registry: &TechnicalTermDictionary,
    existing: &ExistingRefDefs,
) -> Vec<LinkReference> {
    let mut final_defs: Vec<LinkReference> = Vec::new();

    for term in linked_terms {
        let Some(canonical_target) = registry.get(term) else {
            continue;
        };

        if let Some((existing_target, existing_def)) = existing.ref_def_map.get(term)
            && (existing_target == canonical_target
                || existing_target.contains(RUST_PATH_SEPARATOR))
        {
            let expected_label = term_key_to_label(term);
            if existing_def.label == expected_label {
                final_defs.push(existing_def.clone());
            } else {
                final_defs.push(LinkReference {
                    label: expected_label,
                    target: existing_target.clone(),
                    title: existing_def.title.clone(),
                    raw_content: None,
                });
            }
            continue;
        }

        final_defs.push(LinkReference {
            label: term_key_to_label(term),
            target: canonical_target.to_string(),
            title: None,
            raw_content: None,
        });
    }

    for def in &existing.refs {
        let rendered = def.render();
        if let Some((term, _)) = parse_ref_def(&rendered) {
            if !linked_terms.contains(&term) {
                final_defs.push(def.clone());
            }
        } else {
            final_defs.push(def.clone());
        }
    }

    // Sort ref defs alphabetically (case-insensitive) and deduplicate.
    final_defs.sort_by_key(|def| def.render().to_lowercase());
    final_defs.dedup_by(|a, b| a.render() == b.render());

    final_defs
}

/// Checks whether the reference definitions section of a doc block needs to be rewritten.
pub(super) fn should_rewrite_block_ref_defs(
    block: &DocBlockCst,
    existing: &ExistingRefDefs,
    final_defs: &[LinkReference],
    block_modified: bool,
) -> bool {
    let existing_rendered: Vec<String> =
        existing.refs.iter().map(LinkReference::render).collect();
    let final_rendered: Vec<String> =
        final_defs.iter().map(LinkReference::render).collect();

    let refs_layout_canonical = block
        .has_canonical_reference_layout(existing.ref_node_count, !final_defs.is_empty());

    let refs_changed = existing_rendered != final_rendered || !refs_layout_canonical;
    refs_changed || (block_modified && !final_defs.is_empty())
}

/// Converts a registry term key into the [`LinkReference::label`] string (without outer
/// `[` and `]`).
fn term_key_to_label(term: &str) -> String {
    if term.contains(BACKTICK_CHAR) {
        term.to_string()
    } else {
        format!("{BACKTICK_CHAR}{term}{BACKTICK_CHAR}")
    }
}

/// Parses a reference definition line into `(term, target)`.
pub(super) fn parse_ref_def(line: &str) -> Option<(String, String)> {
    let (label, rest) = split_reference_def_label_and_rest(line)?;
    let term = if let Some(after_open) = label.strip_prefix(BACKTICK_CHAR)
        && let Some(inner) = after_open.strip_suffix(BACKTICK_CHAR)
        && !inner.contains(BACKTICK_CHAR)
    {
        inner
    } else {
        label
    };
    Some((term.to_string(), rest.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cargo_rustdoc_fmt::formatters::link_known_terms_for_test;

    fn test_registry() -> TechnicalTermDictionary {
        TechnicalTermDictionary::from_seed(None).unwrap()
    }

    #[test]
    fn test_parse_ref_def_backticked() {
        let (term, target) = parse_ref_def("[`CSI`]: https://example.com").unwrap();
        assert_eq!(term, "CSI");
        assert_eq!(target, "https://example.com");
    }

    #[test]
    fn test_parse_ref_def_plain() {
        let (term, target) = parse_ref_def("[CSI]: https://example.com").unwrap();
        assert_eq!(term, "CSI");
        assert_eq!(target, "https://example.com");
    }

    #[test]
    fn test_parse_ref_def_plain_multiline() {
        // Multi-line ref def combined into "header\n    url".
        let input = "[Inclusive Naming Initiative - Tier 1 Terms]:\n    https://inclusivenaming.org/word-lists/tier-1/";
        let (term, target) = parse_ref_def(input).unwrap();
        assert_eq!(term, "Inclusive Naming Initiative - Tier 1 Terms");
        assert_eq!(target, "https://inclusivenaming.org/word-lists/tier-1/");
    }

    #[test]
    fn test_wrong_target_corrected() {
        let registry = test_registry();
        let input = "Parses [`CSI`] sequences.\n\n[`CSI`]: https://wrong-url.com";
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            result.contains("[`CSI`]: crate::CsiSequence"),
            "Expected corrected target in: {result}"
        );
        assert!(
            !result.contains("wrong-url"),
            "Should not contain wrong URL: {result}"
        );
    }

    #[test]
    fn test_ref_defs_sorted_alphabetically() {
        let registry = test_registry();
        let input = "Uses SGR and CSI and ANSI.";
        let result = link_known_terms_for_test(input, &registry);
        let ref_def_section: String = result
            .lines()
            .skip_while(|l| !l.starts_with('['))
            .collect::<Vec<_>>()
            .join("\n");
        let lines: Vec<&str> = ref_def_section.lines().collect();
        // Should be sorted: ANSI, CSI, SGR.
        assert!(
            lines.len() >= 3,
            "Expected at least 3 ref defs: {ref_def_section}"
        );
        assert!(
            lines[0].contains("ANSI"),
            "First should be ANSI: {ref_def_section}"
        );
        assert!(
            lines[1].contains("CSI"),
            "Second should be CSI: {ref_def_section}"
        );
        assert!(
            lines[2].contains("SGR"),
            "Third should be SGR: {ref_def_section}"
        );
    }

    #[test]
    fn test_existing_intradoc_link_preserved() {
        let registry = test_registry();
        // PTY is in the registry with an external URL, but the file has its own
        // intra-doc link target. The file-local target should be preserved.
        let input = "Spawns a [`PTY`] process.\n\n[`PTY`]: crate::core::pty";
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            result.contains("[`PTY`]: crate::core::pty"),
            "Intra-doc link target should be preserved: {result}"
        );
        assert!(
            !result.contains("wikipedia"),
            "Should not replace with external URL: {result}"
        );
    }

    #[test]
    fn test_plain_ref_def_does_not_overwrite_intradoc_backticked_ref_def() {
        let registry = test_registry();
        let input = concat!(
            "Uses [`termios`] API.\n",
            "\n",
            "[`termios`]: rustix::termios\n",
            "[termios]: https://manpages.debian.org/termios.3",
        );
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            result.contains("[`termios`]: rustix::termios"),
            "Intra-doc backticked ref def should not be overwritten by plain URL ref def: {result}"
        );
    }

    #[test]
    fn test_multiline_ref_defs_preserved_on_second_run() {
        let registry = test_registry();
        // Input with a multi-line ref def (as rustfmt would format it).
        let input =
            concat!("Uses [`CSI`] sequences.\n\n", "[`CSI`]: crate::CsiSequence",);
        let first_run = link_known_terms_for_test(input, &registry);
        let second_run = link_known_terms_for_test(&first_run, &registry);
        assert_eq!(
            first_run, second_run,
            "Term linker should be idempotent:\nFirst:  {first_run}\nSecond: {second_run}"
        );
    }

    #[test]
    fn test_multiline_ref_def_not_linkified_inside_url() {
        let registry = test_registry();
        // The term "epoll" appears inside a URL target. It should NOT be
        // linkified because ref defs are split off before term processing.
        let input =
            "Uses [`epoll`] for I/O.\n\n[`epoll`]: https://manpages.debian.org/epoll.7";
        let result = link_known_terms_for_test(input, &registry);
        assert!(
            result.contains("epoll.7"),
            "URL should not be corrupted: {result}"
        );
        assert!(
            !result.contains("[`epoll`].7"),
            "Term inside URL should not be linkified: {result}"
        );
    }
}
