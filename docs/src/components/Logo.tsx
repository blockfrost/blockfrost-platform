"use client";

import Image from "next/image";
import { useTheme } from "next-themes";

export default function ThemedLogo() {
  const { resolvedTheme } = useTheme();

  const logoSrc =
    resolvedTheme === "dark" ? "/logo-white.svg" : "/logo-black.svg";

  return <Image src={logoSrc} alt="Blockfrost Logo" width={180} height={30} />;
}
