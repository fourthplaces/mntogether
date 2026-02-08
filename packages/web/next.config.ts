import type { NextConfig } from "next";

const nextConfig: NextConfig = {
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

};

export default nextConfig;
