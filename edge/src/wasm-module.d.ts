// wrangler's bundler resolves a `.wasm` import to a compiled
// `WebAssembly.Module`, ready for `WebAssembly.instantiate` — this is what
// `wasm.initSync` in `src/index.ts` expects.
declare module "*.wasm" {
  const module: WebAssembly.Module;
  export default module;
}
