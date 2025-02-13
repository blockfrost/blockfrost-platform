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
      <meta httpEquiv="Content-Type" content="text/html; charset=utf-8" />
      <meta name="viewport" content="width=device-width, initial-scale=1" />
      <title>Blockfrost Platform</title>
      <meta name="title" content=" Blockfrost Platform" />
      <meta
        name="description"
        content="Documentation for Blockfrost Platform"
      />
      <meta
        name="keywords"
        content="Cardano, IPFS, API, Cardano API, SDK, Developers, Ethereum killer, Proof of Stake, NFT, ADA, Lovelace, Shelley, Goguen, Byron, Blockchain, Typescript, Going for #1"
      />
      <meta property="og:type" content="website" />
      <meta property="og:url" content="https://platfrom.blockfrost.io/" />
      <meta property="og:title" content="Blockfrost.io - Cardano API" />
      <meta
        property="og:description"
        content="We provide an instant and scalable Cardano API for free."
      />
      <meta property="og:image" content="https://blockfrost.io/images/og.png" />
      <meta property="twitter:card" content="summary_large_image" />
      <meta property="twitter:url" content="https://platfrom.blockfrost.io/" />
      <meta property="twitter:title" content="Blockfrost Platform" />
      <meta
        property="twitter:description"
        content="We provide an instant and scalable Cardano API for free."
      />
      <meta
        property="twitter:image"
        content="https://blockfrost.io/images/og.png"
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
