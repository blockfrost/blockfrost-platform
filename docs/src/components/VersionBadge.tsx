import packageJson from "../../package.json";

const isNext = process.env.NEXT_PUBLIC_BASE_PATH === "/next";
const channel = isNext ? "next" : "stable";

export default function VersionBadge() {
  return (
    <a
      href={isNext ? "/" : "/next/"}
      className={`version-badge version-badge-${channel}`}
      title={`v${packageJson.version} (${channel}) — switch to ${isNext ? "stable" : "next"}`}
    >
      <span className="version-badge-version">v{packageJson.version}</span>
    </a>
  );
}
