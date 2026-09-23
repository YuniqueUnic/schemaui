//! Content addressing for rich content.

use serde::{Deserialize, Serialize};
#[cfg(feature = "web-types")]
use ts_rs::TS;

use crate::ui_ast::RichContent;

/// A stable id for one [`RichContent`] value: `fnv1a-64` over the variant
/// name, a separator, and the source, hex-encoded.
///
/// Content-addressed on purpose: the same diagram declared on two options
/// renders once and ships once, snapshot output is reproducible across
/// platforms, and the frontend (which computes the identical hash over the
/// identical bytes) can look the rendered asset up without the AST carrying
/// an extra field. The hash is not a security boundary — it only has to
/// collide less often than authors repeat diagrams.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "web-types", derive(TS))]
pub struct RichContentId(pub String);

impl RichContentId {
    pub fn of(content: &RichContent) -> Self {
        // FNV-1a, 64-bit: the offset basis and prime from the reference
        // definition, so the TypeScript twin in the SPA produces the same
        // bytes (locked by shared fixtures in the tests on both sides).
        const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x0000_0100_0000_01b3;

        let mut hash = OFFSET_BASIS;
        let mut absorb = |bytes: &[u8]| {
            for byte in bytes {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(PRIME);
            }
        };
        absorb(content.kind().as_bytes());
        absorb(&[0]);
        absorb(content.source().as_bytes());
        Self(format!("{hash:016x}"))
    }
}
