import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { viteSingleFile } from "vite-plugin-singlefile";

const thisDir = path.dirname(fileURLToPath(import.meta.url));
const workspaceRoot = path.resolve(thisDir, "..");

export default defineConfig(({ mode }) => {
  // Check if building for embedded mode (single-file for Rust embedding)
  const isEmbedded = mode === "embedded";

  return {
    plugins: [
      react(),
      // Only use single-file plugin for embedded builds
      ...(isEmbedded
        ? [viteSingleFile({
          removeViteModuleLoader: true,
          useRecommendedBuildConfig: true,
        })]
        : []),
    ],
    resolve: {
      alias: {
        "@schemaui/types": path.resolve(workspaceRoot, "types"),
        "@": path.resolve(thisDir, "./src"),
      },
    },
    server: {
      fs: {
        allow: [workspaceRoot],
      },
    },
    build: {
      target: "esnext",
      minify: "esbuild",
      sourcemap: !isEmbedded,
      // For embedded: inline everything; for dev: allow code splitting
      assetsInlineLimit: isEmbedded ? 100_000_000 : 4096,
      cssCodeSplit: !isEmbedded,
      outDir: "../dist",
      emptyOutDir: true,
      rollupOptions: {
        output: {
          // Only use inline dynamic imports for embedded builds
          ...(isEmbedded ? { inlineDynamicImports: true } : {}),
        },
      },
    },
  };
});
