import nextra from "nextra";

const withNextra = nextra({
  latex: true,
  search: {
    codeblocks: false,
  },
  contentDirBasePath: "/",
});

const basePath = process.env.NEXT_PUBLIC_BASE_PATH || "";

export default withNextra({
  reactStrictMode: true,
  output: "export",
  ...(basePath && { basePath }),
  images: { unoptimized: true },
});
