/**
 * Content addressing, mirroring `src/rich/id.rs` byte for byte.
 *
 * The Rust side hashes `kind + NUL + source` with FNV-1a 64 and hex-encodes;
 * this twin computes the identical id so a declared figure can be looked up
 * in `SessionResponse.rich` without the AST carrying an extra field. Pinned
 * fixtures on both sides lock the two together — change one, change both.
 */
export function richContentId(kind: "mermaid" | "svg" | "markdown", source: string): string {
  const OFFSET_BASIS = 0xcbf29ce484222325n;
  const PRIME = 0x100000001b3n;
  const MASK = 0xffffffffffffffffn;

  let hash = OFFSET_BASIS;
  const absorb = (bytes: Uint8Array) => {
    for (const byte of bytes) {
      hash ^= BigInt(byte);
      hash = (hash * PRIME) & MASK;
    }
  };
  const encoder = new TextEncoder();
  absorb(encoder.encode(kind));
  absorb(encoder.encode("\0"));
  absorb(encoder.encode(source));
  return hash.toString(16).padStart(16, "0");
}
