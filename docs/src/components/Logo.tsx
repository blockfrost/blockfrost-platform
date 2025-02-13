"use client";

import Image from "next/image";
import { useTheme } from "next-themes";
import { useEffect, useState } from "react";

export default function Logo() {
  const { resolvedTheme } = useTheme();
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  if (!mounted) return null;

  const logoSrc =
    resolvedTheme === "dark" ? "/logo-white.svg" : "/logo-black.svg";

  return <Image src={logoSrc} alt="Blockfrost Logo" width={180} height={30} />;
}
