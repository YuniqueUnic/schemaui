// This file mirrors `src/rich/asset.rs::RichAsset` (generated twin lives in
// `bindings/RichAsset.ts`); hand-maintained like SessionResponse so the doc
// comments survive regeneration.

/**
 * An SVG plus its intrinsic size, when the source declares one. The size
 * comes from the root `width`/`height`/`viewBox` and lets a thumbnail keep
 * the drawing's aspect ratio.
 */
export interface RenderedSvg {
    body: string;
    width: number | null;
    height: number | null;
}

/**
 * The rendered, sanitised form of one declared rich content, addressed by
 * content id in `SessionResponse.rich`.
 *
 * `failed` reports an error in the author's source and is shown next to it;
 * a kind this build cannot render is *absent* from the map instead — a
 * capability question, answered by falling back to the source.
 */
export type RichAsset =
    | { kind: "diagram"; light: RenderedSvg; dark: RenderedSvg }
    | { kind: "svg"; svg: RenderedSvg }
    | { kind: "html"; html: string }
    | { kind: "failed"; message: string };
