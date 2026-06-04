"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { api, MarketSummary } from "@/lib/api";

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
      <div style={{ marginBottom: 28 }}>
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
      </div>

      <Row label="Expires in" value={timeUntil(market.expiry)} />
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
