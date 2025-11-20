import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "path";

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // Vite options tailored for Tauri development
  clearScreen: false,

  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },

  resolve: {
    alias: {
      "@": resolve(__dirname, "./src"),
      "@components": resolve(__dirname, "./src/components"),
      "@hooks": resolve(__dirname, "./src/hooks"),
      "@services": resolve(__dirname, "./src/services"),
      "@types": resolve(__dirname, "./src/types"),
      "@contexts": resolve(__dirname, "./src/contexts"),
      "@styles": resolve(__dirname, "./src/styles"),
    },
  },

  // Tauri expects a fixed port, fail if that port is not available
  build: {
    target: process.env.TAURI_PLATFORM == "windows" ? "chrome105" : "safari13",

    // Always use esbuild minify for consistent builds
    // This ensures reproducible output regardless of environment
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,

    // Enable sourcemaps for debugging
    // Helps troubleshoot bundling issues
    sourcemap: true,

    // CRITICAL: Always clean dist/ directory before building
    // Prevents Vite caching issues where old code gets bundled
    emptyOutDir: true,

    rollupOptions: {
      external: ['@vite/client'],
      output: {
        // Add content hash to filenames for cache busting
        // Ensures browsers don't cache stale JavaScript
        entryFileNames: 'assets/[name]-[hash].js',
        chunkFileNames: 'assets/[name]-[hash].js',
        assetFileNames: 'assets/[name]-[hash].[ext]'
      },
    },
  },

  // Force dependency re-optimization
  // Prevents stale dependency caching
  optimizeDeps: {
    force: true,
  },
}));
