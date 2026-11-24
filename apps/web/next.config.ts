import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Keep the dev server's HMR chunks separate from production build output.
  // Running `next build` while `next dev` is open otherwise corrupts the
  // browser's module factory table and can surface as `factory.call` errors.
  distDir: process.env.NODE_ENV === "development" ? ".next-dev" : ".next",
  transpilePackages: ["@forge/ui", "@forge/api-client", "@forge/types"],
};

export default nextConfig;
