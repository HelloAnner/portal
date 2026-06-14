import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  output: "export",
  distDir: "out",
  allowedDevOrigins: ["127.0.0.1", "localhost"],
};

export default nextConfig;
