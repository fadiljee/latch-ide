import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  build: {
    outDir: "dist",
    sourcemap: false,
    rollupOptions: {
      input: {
        main: new URL("./index.html", import.meta.url).pathname,
        floating: new URL("./floating.html", import.meta.url).pathname,
      },
      output: {
        manualChunks: {
          vendor: ["react", "react-dom", "zustand"],
        },
      },
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: "./src/test/setup.ts",
  },
});
