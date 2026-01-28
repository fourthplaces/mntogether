import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  base: '/admin/',
  server: {
    port: 3000,
    proxy: {
      '/graphql': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
    },
  },
})
