//! Content addressing.
//!
//! The exact hex values below are contract fixtures, not just stability
//! checks: the SPA computes the same hash in TypeScript to look assets up,
//! and a drift on either side strands every figure on the other. Change the
//! hash, change both files.

use crate::rich::RichContentId;
use crate::ui_ast::RichContent;

fn mermaid(source: &str) -> RichContent {
    RichContent::Mermaid {
        source: source.to_string(),
    }
}

#[test]
fn fixture_vectors_are_pinned() {
    assert_eq!(
        RichContentId::of(&mermaid("flowchart LR; A-->B")).0,
        "4e445939aa84dc36"
    );
    assert_eq!(
        RichContentId::of(&RichContent::Svg {
            source: "<svg/>".to_string()
        })
        .0,
        "998e073cd59a6064"
    );
    assert_eq!(
        RichContentId::of(&RichContent::Markdown {
            source: "# hi".to_string()
        })
        .0,
        "ae1b2b51e60236b4"
    );
}

#[test]
fn same_content_same_id_regardless_of_history() {
    assert_eq!(
        RichContentId::of(&mermaid("flowchart TD; A-->B")),
        RichContentId::of(&mermaid("flowchart TD; A-->B"))
    );
}

#[test]
fn kind_participates_in_the_hash() {
    // The same bytes as a diagram source and as an SVG figure are different
    // content and must never share an entry.
    assert_ne!(
        RichContentId::of(&mermaid("flowchart LR; A-->B")),
        RichContentId::of(&RichContent::Svg {
            source: "flowchart LR; A-->B".to_string()
        })
    );
}

#[test]
fn ids_are_sixteen_lowercase_hex_digits() {
    let id = RichContentId::of(&mermaid("x"));
    assert_eq!(id.0.len(), 16);
    assert!(
        id.0.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    );
}
