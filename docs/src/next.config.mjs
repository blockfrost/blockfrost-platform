import nextra from "nextra";

const withNextra = nextra({
  latex: true,
  search: {
    codeblocks: false,
  },
  i18n: {
    locales: ["en"],
    defaultLocale: "en",
  },
  theme: "nextra-theme-docs",
  contentDirBasePath: "content",
});

export default withNextra({
  reactStrictMode: true,
});
