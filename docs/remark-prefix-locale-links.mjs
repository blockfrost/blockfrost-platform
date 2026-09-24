const LOCALES = ["en", "ja"];
const LOCALE_RE = new RegExp(`^/(${LOCALES.join("|")})(/|$|#|\\?)`);

const visit = (node, locale) => {
  if (node.type === "link" && typeof node.url === "string") {
    const url = node.url;
    if (url.startsWith("/") && !url.startsWith("//") && !LOCALE_RE.test(url)) {
      node.url = `/${locale}${url}`;
    }
  }
  if (node.children) {
    for (const child of node.children) visit(child, locale);
  }
};

export default function remarkPrefixLocaleLinks() {
  return (tree, file) => {
    const fp = file.history?.[0] || file.path || "";
    const m = fp.match(/[\\/]content[\\/]([^\\/]+)[\\/]/);
    if (!m) return;
    const locale = m[1];
    if (!LOCALES.includes(locale)) return;
    visit(tree, locale);
  };
}
