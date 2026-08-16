import type { NextConfig } from "next";

const allowedDevOrigins = process.env.NEXT_ALLOWED_DEV_ORIGINS?.split(",")
  .map((origin) => origin.trim())
  .filter(Boolean);

const canonicalLocalOrigin =
  process.env.WEBUI_CANONICAL_ORIGIN?.trim() ||
  (process.env.NODE_ENV === "development" ? "http://127.0.0.1:3000" : undefined);

const nextConfig: NextConfig = {
  // Emit a self-contained server bundle (.next/standalone) so the Docker runtime
  // stage ships only what the server needs — no node_modules, no build toolchain.
  output: "standalone",
  ...(allowedDevOrigins?.length ? { allowedDevOrigins } : {}),
  async redirects() {
    if (!canonicalLocalOrigin) return [];

    const canonicalUrl = new URL(canonicalLocalOrigin);
    if (canonicalUrl.hostname !== "127.0.0.1") return [];

    return [
      {
        source: "/:path*",
        has: [{ type: "host", value: "localhost" }],
        destination: `${canonicalUrl.origin}/:path*`,
        permanent: false,
      },
    ];
  },
};

export default nextConfig;
