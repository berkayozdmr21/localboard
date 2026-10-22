import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
export default defineConfig({
  plugins: [react()],
  server: {
    strictPort: true,
    proxy: {
      "/api": {
        target: "http://127.0.0.1:9140",
        changeOrigin: true,
        headers: { Origin: "http://127.0.0.1:9140" },
      },
    },
  },
  test: { environment: "jsdom", globals: true },
});
