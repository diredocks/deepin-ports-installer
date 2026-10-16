import tailwindcss from "@tailwindcss/vite";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import solidPlugin from "vite-plugin-solid";

// https://vitejs.dev/config
export default defineConfig({
  root: path.resolve(__dirname, "src/app"),
  resolve: {
    alias: {
      "~": fileURLToPath(new URL("./src/app", import.meta.url)),
    },
  },
  plugins: [solidPlugin(), tailwindcss()],
});
