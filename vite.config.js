import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

const root = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  root: path.join(root, "frontend"),
  publicDir: path.join(root, "public"),
  base: "./",
  build: {
    outDir: path.join(root, "dist"),
    emptyOutDir: true,
  },
  clearScreen: false,
  server: {
    strictPort: true,
  },
});
