import nextConfig from "eslint-config-next";
import prettier from "eslint-config-prettier";

const eslintConfig = [
  ...nextConfig,
  prettier,
  {
    ignores: [".next/**", ".yarn/**", "node_modules/**", "out/**", "public/**"],
  },
  {
    files: ["**/_meta.js"],
    rules: {
      "import/no-anonymous-default-export": "off",
    },
  },
  {
    rules: {
      "no-dupe-keys": "off",
    },
  },
];

export default eslintConfig;
