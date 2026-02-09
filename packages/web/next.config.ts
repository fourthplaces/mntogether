import type { NextConfig } from "next";

const securityHeaders = [
  { key: "X-Frame-Options", value: "DENY" },
  { key: "X-Content-Type-Options", value: "nosniff" },
  { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
  { key: "X-XSS-Protection", value: "1; mode=block" },
];

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
