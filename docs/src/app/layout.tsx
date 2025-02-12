import { Footer, Layout, Navbar } from "nextra-theme-docs";
import { Head } from "nextra/components";
import { getPageMap } from "nextra/page-map";
import "nextra-theme-docs/style.css";
import Image from "next/image";

export const metadata = {
  metadataBase: new URL("https://nextra.site"),
  title: {
    template: "%s - Documentation",
  },
  description: "Nextra: the Next.js site builder",
  applicationName: "Nextra",
  generator: "Next.js",
  appleWebApp: {
    title: "Nextra",
  },
  other: {
    "msapplication-TileImage": "/ms-icon-144x144.png",
    "msapplication-TileColor": "#fff",
  },
  discord: {
    site: "https://discord.gg/inputoutput",
  },
  twitter: {
    site: "https://nextra.site",
  },
};

export default async function RootLayout({ children }) {
  const navbar = (
    <Navbar
      logo={
        <Image
          width={180}
          height={30}
          alt="Blockfrost Logo"
          src="https://blockfrost.dev/img/logo.svg"
        />
      }
      projectLink="https://github.com/blockfrost/blockfrost-platform"
      chatLink="https://discord.gg/inputoutput"
    />
  );
  const pageMap = await getPageMap();
  return (
    <html lang="en" dir="ltr" suppressHydrationWarning>
      <Head faviconGlyph="✦" />
      <body>
        <Layout
          navbar={navbar}
          footer={<Footer>MIT {new Date().getFullYear()} © Nextra.</Footer>}
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
