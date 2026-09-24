export const metadata = {
  title: "Blockfrost Platform Documentation",
  robots: { index: false, follow: false },
};

const basePath = process.env.NEXT_PUBLIC_BASE_PATH || "";

export default function RootRedirect() {
  return (
    <>
      <style
        dangerouslySetInnerHTML={{
          __html: "html{visibility:hidden!important;background:#000}",
        }}
      />
      <script
        dangerouslySetInnerHTML={{
          __html: `(function(){try{var l=(navigator.language||'').toLowerCase();location.replace(l.indexOf('ja')===0?'${basePath}/ja':'${basePath}/en')}catch(e){location.replace('${basePath}/en')}})();`,
        }}
      />
      <noscript>
        <meta httpEquiv="refresh" content={`0; url=${basePath}/en`} />
      </noscript>
    </>
  );
}
