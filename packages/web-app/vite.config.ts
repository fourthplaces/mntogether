import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

// Use 'api' (Docker service name) when running in container, 'localhost' when running locally
const apiTarget = process.env.DOCKER_ENV === 'true' ? 'http://api:8080' : 'http://localhost:8080'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
    extensions: ['.js', '.jsx', '.ts', '.tsx', '.json'],
  },
  server: {
    port: 3001,
    proxy: {
      '/graphql': {
        target: apiTarget,
        changeOrigin: true,
      },
    },
  },
})
