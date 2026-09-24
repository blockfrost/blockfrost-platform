import nextra from "nextra";
import remarkPrefixLocaleLinks from "./remark-prefix-locale-links.mjs";

const withNextra = nextra({
  latex: true,
  search: {
    codeblocks: false,
  },
  contentDirBasePath: "/",
  mdxOptions: {
    remarkPlugins: [remarkPrefixLocaleLinks],
  },
});

const basePath = process.env.NEXT_PUBLIC_BASE_PATH || "";

export default withNextra({
  reactStrictMode: true,
  output: "export",
  ...(basePath && { basePath }),
  images: { unoptimized: true },
  i18n: {
    locales: ["en", "ja"],
    defaultLocale: "en",
  },
});
