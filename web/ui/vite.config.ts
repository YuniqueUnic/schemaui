import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { viteSingleFile } from "vite-plugin-singlefile";

const thisDir = path.dirname(fileURLToPath(import.meta.url));
const workspaceRoot = path.resolve(thisDir, "..");
const repoRoot = path.resolve(workspaceRoot, "..");

export default defineConfig(({ mode }) => {
  // Check if building for embedded mode (single-file for Rust embedding)
  const isEmbedded = mode === "embedded";
  // The static, server-less Playground: a separate entry (playground.html)
  // built to its own output directory, deployed to GitHub Pages.
  const isPlayground = mode === "playground";

  return {
    ...(isEmbedded ? {
      base: "./",
      publicDir: false,
    } : {}),
    // Relative asset paths, so the built site works from whatever subpath it
    // is served under (a GitHub Pages project page, a custom domain, ...)
    // without a repo name baked into the config.
    ...(isPlayground ? { base: "./" } : {}),
    plugins: [
      react(),
      // Only use single-file plugin for embedded builds
      ...(isEmbedded
        ? [viteSingleFile({
          removeViteModuleLoader: true,
          useRecommendedBuildConfig: false,
        })]
        : []),
    ],
    resolve: {
      alias: {
        "@schemaui/types": path.resolve(workspaceRoot, "types"),
        // The built wasm package (`just build-wasm`), consumed by the
        // Playground's WasmBackend. Not an npm dependency: aliasing straight
        // to the workspace build output avoids a pnpm-install-order
        // dependency on a directory that only exists after a wasm-pack build.
        "@schemaui/wasm": path.resolve(repoRoot, "schemaui-wasm/pkg/schemaui_wasm.js"),
        "@": path.resolve(thisDir, "./src"),
      },
    },
    server: {
      fs: {
        allow: [repoRoot],
      },
      // `pnpm dev` serves the SPA on its own port; without this, every
      // `/api/*` call it makes targets that same port and 404s, because
      // nothing here is a `schemaui web` session. Proxying — not a CORS
      // layer — keeps the request same-origin from the browser's point of
      // view and needs no change on the server. Not for `playground` mode:
      // the Playground's WasmBackend never calls `/api/*` at all, so a proxy
      // there would suggest a capability that mode deliberately does not have.
      ...(isPlayground ? {} : {
        proxy: {
          "/api": {
            target: "http://127.0.0.1:8787",
            changeOrigin: true,
          },
        },
      }),
    },
    build: {
      target: "esnext",
      minify: "oxc",
      sourcemap: !isEmbedded,
      // For embedded: inline everything; for dev: allow code splitting
      assetsInlineLimit: isEmbedded ? () => true : 4096,
      ...(isEmbedded ? {
        assetsDir: "",
        chunkSizeWarningLimit: 100_000_000,
      } : {}),
      cssCodeSplit: !isEmbedded,
      outDir: isPlayground ? "../playground-dist" : "../dist",
      emptyOutDir: true,
      ...(isPlayground ? {
        rollupOptions: {
          input: path.resolve(thisDir, "playground.html"),
        },
      } : {}),
      ...(isEmbedded ? {
        rolldownOptions: {
          output: {
            codeSplitting: false,
          },
        },
      } : {}),
    },
    test: {
      environment: "jsdom",
      setupFiles: "./src/test/setup.ts",
    },
  };
});
