import { Layout, Navbar } from "nextra-theme-docs";
import { Head } from "nextra/components";
import { getPageMap } from "nextra/page-map";
import "nextra-theme-docs/style.css";
import "../styles.css";
import Logo from "../components/Logo";

export const metadata = {
  metadataBase: new URL("https://platform.blockfrost.io"),
  title: {
    template: "%s - Documentation",
  },
  description: "Blockfrost Platform Documentation",
  applicationName: "Blockfrost Platform",
  generator: "Next.js",
  appleWebApp: {
    title: "Blockfrost Platform",
  },
  discord: {
    site: "https://discord.gg/inputoutput",
  },
  twitter: {
    site: "https://x.com/blockfrost_io",
  },
};

const Footer = ({ children }) => (
  <footer className="footer">
    <div className="footer-content">{children}</div>
  </footer>
);

export default async function RootLayout({ children }) {
  const navbar = (
    <Navbar
      logo={<Logo />}
      projectLink="https://github.com/blockfrost/blockfrost-platform"
      chatLink="https://discord.gg/inputoutput"
    />
  );
  const pageMap = await getPageMap();

  return (
    <html lang="en" dir="ltr" suppressHydrationWarning>
      <Head faviconGlyph="✦" />
      <meta property="og:title" content="Blockfrost Platform Documentation" />
      <meta
        property="og:description"
        content="Blockfrost Platform Documentation"
      />
      <meta property="og:url" content="https://platform.blockfrost.io" />
      <meta
        property="og:image"
        content="https://platform.blockfrost.io/logo-black.svg"
      />
      <meta property="og:type" content="website" />
      <meta property="og:site_name" content="Blockfrost Platform" />
      <meta name="twitter:card" content="summary_large_image" />
      <meta name="twitter:site" content="@blockfrost_io" />
      <meta name="twitter:title" content="Blockfrost Platform Documentation" />
      <meta
        name="twitter:description"
        content="Blockfrost Platform Documentation"
      />
      <meta
        name="twitter:image"
        content="https://platform.blockfrost.io/logo-black.svg"
      />

      <body>
        <div className="flare"></div>
        <Layout
          navbar={navbar}
          footer={<Footer>{new Date().getFullYear()} © Blockfrost.</Footer>}
          editLink="https://github.com/blockfrost/blockfrost-platform"
          docsRepositoryBase="https://github.com/blockfrost/blockfrost-platform/docs"
          sidebar={{ defaultMenuCollapseLevel: 1 }}
          pageMap={pageMap}
        >
          {children}
        </Layout>
      </body>
    </html>
  );
}
