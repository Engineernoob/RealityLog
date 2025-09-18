import { defineConfig } from "vite";

export default defineConfig({
  server: {
    host: "127.0.0.1",
    port: 5173,
    fs: {
      // allow access to the wasm-core/pkg folder outside of verifier-ext
      allow: ["..", "../wasm-core/pkg"],
    },
  },
});
