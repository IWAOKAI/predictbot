"use client";

import dynamic from "next/dynamic";

// dapp-kit は React Context を使うため SSR では動かない。
// ssr: false で完全にクライアント専用にする。
const Providers = dynamic(
  () => import("./providers").then((m) => m.Providers),
  { ssr: false }
);

export function ClientProviders({ children }: { children: React.ReactNode }) {
  return <Providers>{children}</Providers>;
}
