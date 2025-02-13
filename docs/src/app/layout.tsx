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
  description: "Documentation for Blockfrost Platform",
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
      <Head faviconGlyph="✦">
        <meta httpEquiv="Content-Type" content="text/html; charset=utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <title>Blockfrost Platform Documentation</title>
        <meta name="title" content="Blockfrost Platform Documentation" />
        <meta
          name="description"
          content="Documentation for Blockfrost Platform. The Blockfrost platform transforms your Cardano node infrastructure into a high-performance JSON API endpoint, offering deployment options to join the fleet or run independently."
        />
        <meta
          name="keywords"
          content="Blockfrost, Cardano, Documentation, JSON API, Stake Pool Operator, Node Operator, decentralized, API"
        />
        <meta property="og:type" content="website" />
        <meta property="og:url" content="https://platform.blockfrost.io/" />
        <meta property="og:title" content="Blockfrost Platform Documentation" />
        <meta
          property="og:description"
          content="Transform your Cardano node infrastructure into a high-performance JSON API endpoint with Blockfrost. Learn about deployment options to join the fleet or run independently."
        />
        <meta
          property="og:image"
          content="https://blockfrost.io/images/og.png"
        />
        <meta name="twitter:card" content="summary_large_image" />
        <meta name="twitter:url" content="https://platform.blockfrost.io/" />
        <meta
          name="twitter:title"
          content="Blockfrost Platform Documentation"
        />
        <meta
          name="twitter:description"
          content="Transform your Cardano node infrastructure into a high-performance JSON API endpoint with Blockfrost. Learn about deployment options to join the fleet or run independently."
        />
        <meta
          name="twitter:image"
          content="https://blockfrost.io/images/og.png"
        />
        <link rel="icon" href="/favicon.ico" />
      </Head>
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
