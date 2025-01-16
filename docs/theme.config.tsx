import React from "react";
import { DocsThemeConfig } from "nextra-theme-docs";

const config: DocsThemeConfig = {
  logo: () => (
    <img src='https://blockfrost.dev/img/logo.svg' style={{ "height": "30px" }} />
  ),
  project: {
    link: "https://github.com/blockfrost/blockfrost-platform",
  },
  chat: {
    link: "https://discord.gg/inputoutput",
  },
  docsRepositoryBase: "https://github.com/blockfrost/blockfrost-platform/tree/main/docs",
  footer: {},
};

export default config;
