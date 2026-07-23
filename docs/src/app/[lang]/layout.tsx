import { Layout, LocaleSwitch, Navbar } from "nextra-theme-docs";
import { Head } from "nextra/components";
import { getPageMap } from "nextra/page-map";
import Logo from "../../components/Logo";
import VersionBadge from "../../components/VersionBadge";

const Footer = ({ children }) => (
  <footer className="footer">
    <div className="footer-content">{children}</div>
  </footer>
);

export async function generateStaticParams() {
  return [{ lang: "en" }, { lang: "ja" }];
}

const prefixPageMapRoutes = (items, lang) =>
  items.map(item => {
    const next = { ...item };
    if (
      typeof next.route === "string" &&
      next.route.startsWith("/") &&
      !next.route.startsWith(`/${lang}`)
    ) {
      next.route = `/${lang}${next.route}`;
    }
    if (Array.isArray(next.children)) {
      next.children = prefixPageMapRoutes(next.children, lang);
    }
    return next;
  });

export default async function LangLayout({ children, params }) {
  const { lang } = await params;
  const navbar = (
    <Navbar
      logo={<Logo />}
      projectLink="https://github.com/blockfrost/blockfrost-platform"
      chatLink="https://discord.gg/YfGKvqpt2d"
    >
      <VersionBadge />
      <LocaleSwitch lite />
    </Navbar>
  );

  const rawPageMap = await getPageMap(`/${lang}`);
  const pageMap = prefixPageMapRoutes(rawPageMap, lang);

  return (
    <>
      <Head>
        <meta property="og:type" content="website" />
        <meta property="og:url" content="https://platform.blockfrost.io/" />
        <meta property="og:title" content="Blockfrost Platform Documentation" />
        <meta
          property="og:image"
          content="https://blockfrost.io/images/og.png"
        />
        <meta name="twitter:card" content="summary_large_image" />
        <meta
          name="twitter:image"
          content="https://blockfrost.io/images/og.png"
        />
      </Head>
      <script
        dangerouslySetInnerHTML={{
          __html: `document.documentElement.lang=${JSON.stringify(lang)};`,
        }}
      />
      <Layout
        navbar={navbar}
        footer={<Footer>{new Date().getFullYear()} © Blockfrost.</Footer>}
        editLink="https://github.com/blockfrost/blockfrost-platform"
        docsRepositoryBase="https://github.com/blockfrost/blockfrost-platform/docs"
        sidebar={{ defaultMenuCollapseLevel: 1 }}
        pageMap={pageMap}
        i18n={[
          { locale: "en", name: "English" },
          { locale: "ja", name: "日本語" },
        ]}
      >
        {children}
      </Layout>
    </>
  );
}
