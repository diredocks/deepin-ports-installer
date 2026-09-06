import { defineConfig } from "vite";
import path from "node:path";
import { fileURLToPath } from "node:url";

// https://vitejs.dev/config
export default defineConfig({
  root: path.resolve(__dirname, "src/app"),
  resolve: {
    alias: {
      "~": fileURLToPath(new URL("./src/app", import.meta.url)),
    },
  },
});
