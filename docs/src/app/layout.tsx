import "nextra-theme-docs/style.css";
import "../styles.css";

export const metadata = {
  metadataBase: new URL("https://platform.blockfrost.io"),
  title: {
    template: "%s - Documentation",
    default: "Blockfrost Platform Documentation",
  },
  description: "Documentation for Blockfrost platform",
  applicationName: "Blockfrost platform",
  generator: "Next.js",
  appleWebApp: {
    title: "Blockfrost platform",
  },
};

export default function RootLayout({ children }) {
  return (
    <html dir="ltr" suppressHydrationWarning>
      <head>
        <link rel="icon" href="/favicon.ico" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <meta
          name="keywords"
          content="Blockfrost, Cardano, Documentation, JSON API, Stake Pool Operator, Node Operator, decentralized, API"
        />
      </head>
      <body>
        <div className="flare"></div>
        {children}
      </body>
    </html>
  );
}
