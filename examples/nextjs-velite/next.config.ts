import type { NextConfig } from "next";

const config: NextConfig = {
  // Velite's Next.js plugin auto-runs the build step on dev/build, but we
  // also keep the explicit `pnpm content` predev/prebuild script so the
  // example works without the plugin.
};

export default config;
