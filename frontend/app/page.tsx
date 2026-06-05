"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { ConnectButton, useCurrentAccount } from "@mysten/dapp-kit";
import { api, MarketSummary } from "@/lib/api";

function minsUntil(expiryMs: number): number {
  return Math.floor((expiryMs - Date.now()) / 60000);
}

function timeUntil(expiryMs: number): string {
  const diff = expiryMs - Date.now();
  if (diff <= 0) return "expired";
  const mins = Math.floor(diff / 60000);
  if (mins < 60) return `${mins}m`;
  const hrs = Math.floor(mins / 60);
  return `${hrs}h ${mins % 60}m`;
}

export default function MarketsPage() {
  const [markets, setMarkets] = useState<MarketSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .markets()
      .then((res) => {
        const active = res.markets
          .filter((m) => m.status === "active")
          .sort((a, b) => a.expiry - b.expiry);
        setMarkets(active);
      })
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, []);

  return (
    <div>
      <div
        style={{
          marginBottom: 28,
          display: "flex",
          justifyContent: "space-between",
          alignItems: "flex-start",
          gap: 16,
          flexWrap: "wrap",
        }}
      >
        <div>
          <h1
            style={{
              fontSize: 32,
              fontWeight: 800,
              margin: "0 0 6px",
              letterSpacing: "-0.02em",
              color: "var(--primary-dark)",
            }}
          >
            Don&apos;t Bet Blind. See the Math.
          </h1>
          <p style={{ color: "var(--text-muted)", margin: 0, fontSize: 15 }}>
            Live DeepBook Predict markets, priced against an SVI fair-value model.
          </p>
        </div>
        <WalletBar />
      </div>

      {loading && <LoadingState />}
      {error && <ErrorState message={error} />}

      {!loading && !error && (
        <>
          <div
            style={{
              fontSize: 13,
              color: "var(--text-muted)",
              marginBottom: 16,
              fontWeight: 600,
            }}
          >
            {markets.length} active markets
          </div>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))",
              gap: 16,
            }}
          >
            {markets.map((m) => (
              <MarketCard key={m.oracle_id} market={m} />
            ))}
          </div>
        </>
      )}
    </div>
  );
}

function MarketCard({ market }: { market: MarketSummary }) {
  return (
    <Link
      href={`/market/${market.oracle_id}`}
      style={{ textDecoration: "none", color: "inherit" }}
    >
    <div className="card" style={{ padding: 20, cursor: "pointer" }}>
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: 14,
        }}
      >
        <span
          style={{
            fontWeight: 800,
            fontSize: 18,
            color: "var(--primary-dark)",
          }}
        >
          {market.underlying_asset}
        </span>
        <span className="badge badge-active">active</span>
        {minsUntil(market.expiry) < 5 && (
          <span
            style={{
              fontSize: 11,
              fontWeight: 700,
              color: "#b45309",
              background: "#fef3c7",
              padding: "2px 8px",
              borderRadius: 999,
            }}
          >
            closing soon
          </span>
        )}
      </div>

      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          padding: "6px 0",
          fontSize: 13,
        }}
      >
        <span style={{ color: "var(--text-muted)" }}>Expires in</span>
        <span
          style={{
            fontWeight: 600,
            color: minsUntil(market.expiry) < 5 ? "var(--down)" : "var(--text)",
          }}
        >
          {timeUntil(market.expiry)}
        </span>
      </div>
      <Row
        label="Min strike"
        value={`$${market.min_strike_usd.toLocaleString()}`}
      />
      <Row label="Tick size" value={`$${market.tick_size_usd}`} />

      <div
        style={{
          marginTop: 14,
          paddingTop: 14,
          borderTop: "1px solid var(--border)",
          fontSize: 11,
          color: "var(--text-muted)",
          fontFamily: "ui-monospace, monospace",
          wordBreak: "break-all",
        }}
      >
        {market.oracle_id.slice(0, 18)}…
      </div>
    </div>
    </Link>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div
      style={{
        display: "flex",
        justifyContent: "space-between",
        fontSize: 14,
        padding: "4px 0",
      }}
    >
      <span style={{ color: "var(--text-muted)" }}>{label}</span>
      <span style={{ fontWeight: 600 }}>{value}</span>
    </div>
  );
}

function WalletBar() {
  const account = useCurrentAccount();
  return (
    <div style={{ display: "flex", flexDirection: "column", alignItems: "flex-end", gap: 6 }}>
      <ConnectButton />
      {account && (
        <span
          style={{
            fontSize: 11,
            color: "var(--text-muted)",
            fontFamily: "ui-monospace, monospace",
          }}
        >
          {account.address.slice(0, 10)}…{account.address.slice(-4)}
        </span>
      )}
    </div>
  );
}

function LoadingState() {
  return (
    <div
      style={{
        padding: 48,
        textAlign: "center",
        color: "var(--text-muted)",
      }}
    >
      Loading live markets…
    </div>
  );
}

function ErrorState({ message }: { message: string }) {
  return (
    <div
      className="card"
      style={{ padding: 24, borderColor: "var(--down)", color: "var(--down)" }}
    >
      <strong>Could not reach DeepEdge API.</strong>
      <div style={{ fontSize: 13, marginTop: 6, color: "var(--text-muted)" }}>
        {message} — is the backend running on :3000?
      </div>
    </div>
  );
}
