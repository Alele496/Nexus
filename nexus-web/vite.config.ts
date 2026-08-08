import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

// Dev server proxies the ACP WebSocket to the agent server so the browser can
// connect with just `npm run dev` (no server key in the proxy path — dev is
// expected to hit a local `nexus agent serve`).
export default defineConfig({
  plugins: [react(), tailwindcss()],
  build: {
    emptyOutDir: true,
  },
  server: {
    port: 5173,
    proxy: {
      '/ws': {
        target: 'ws://127.0.0.1:2419',
        ws: true,
        changeOrigin: true,
      },
    },
  },
});
