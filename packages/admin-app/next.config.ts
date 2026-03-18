import type { NextConfig } from "next";
import { resolve } from "path";

const securityHeaders = [
  { key: "X-Frame-Options", value: "DENY" },
  { key: "X-Content-Type-Options", value: "nosniff" },
  { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
  { key: "X-XSS-Protection", value: "1; mode=block" },
];

const nextConfig: NextConfig = {
  // Monorepo root — helps Turbopack resolve hoisted node_modules
  outputFileTracingRoot: resolve(process.cwd(), "../.."),

  // Transpile shared package
  transpilePackages: [
    "@rooteditorial/shared",
    "platejs",
    "@platejs/core",
    "@platejs/basic-nodes",
    "@platejs/link",
    "@platejs/list",
    "@platejs/markdown",
    "@platejs/slate",
    "@platejs/utils",
  ],

  // Keep GraphQL server packages as external (Node.js runtime, not bundled)
  serverExternalPackages: [
    "graphql-yoga",
    "graphql",
    "@graphql-tools/schema",
    "@graphql-tools/merge",
    "@graphql-tools/utils",
    "dataloader",
  ],

  // Optimize images
  images: {
    domains: [],
    formats: ["image/avif", "image/webp"],
  },

  // Strict mode
  reactStrictMode: true,

  // Disable telemetry
  typescript: {
    ignoreBuildErrors: false,
  },

  // Turbopack doesn't resolve package.json "exports" subpaths for hoisted
  // monorepo packages. Map subpath imports to their actual dist files.
  // Paths must work both locally (../../node_modules) and in Docker (/app/node_modules).
  turbopack: {
    resolveAlias: {
      "@platejs/basic-nodes/react": "@platejs/basic-nodes/dist/react/index.js",
      "@platejs/link/react": "@platejs/link/dist/react/index.js",
      "@platejs/list/react": "@platejs/list/dist/react/index.js",
      "platejs/react": "platejs/dist/react/index.js",
    },
  },

  async headers() {
    return [
      {
        source: "/(.*)",
        headers: securityHeaders,
      },
    ];
  },
};

export default nextConfig;
