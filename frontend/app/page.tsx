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
  const [fairMap, setFairMap] = useState<Record<string, number>>({});
  const [atmMap, setAtmMap] = useState<Record<string, number>>({});
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
        // Fetch each market's ATM fair P(up) in parallel, so we can flag
        // the largest-edge market and the calibration-caution bucket.
        Promise.allSettled(active.map((m) => api.strikes(m.oracle_id))).then(
          (results) => {
            const fm: Record<string, number> = {};
            const am: Record<string, number> = {};
            results.forEach((r, i) => {
              if (r.status === "fulfilled") {
                const g = r.value.grid;
                const atm = g.strikes.reduce((best, row) =>
                  Math.abs(row.strike_usd - g.atm_strike_usd) <
                  Math.abs(best.strike_usd - g.atm_strike_usd) ? row : best,
                  g.strikes[0]);
                fm[active[i].oracle_id] = atm.fair_up;
                am[active[i].oracle_id] = g.atm_strike_usd;
              }
            });
            setFairMap(fm);
            setAtmMap(am);
          }
        );
      })
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, []);

  // Which active market has the largest edge (fair furthest from 0.50)?
  const largestEdgeId = (() => {
    let id: string | null = null;
    let best = -1;
    for (const [oid, fair] of Object.entries(fairMap)) {
      const d = Math.abs(fair - 0.5);
      if (d > best) { best = d; id = oid; }
    }
    return id;
  })();

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
              <MarketCard key={m.oracle_id} market={m} fair={fairMap[m.oracle_id]} atm={atmMap[m.oracle_id]} isLargestEdge={m.oracle_id === largestEdgeId} />
            ))}
          </div>
        </>
      )}
    </div>
  );
}

function MarketCard({ market, fair, atm, isLargestEdge }: { market: MarketSummary; fair?: number; atm?: number; isLargestEdge?: boolean }) {
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
        {isLargestEdge && (
          <span
            style={{
              fontSize: 11, fontWeight: 700, color: "#7c2d12",
              background: "#fed7aa", padding: "2px 8px", borderRadius: 999,
            }}
            title="Fair probability is furthest from 50/50 - the agent's pick"
          >
            largest edge
          </span>
        )}
        {isLargestEdge && fair !== undefined && fair >= 0.4 && fair < 0.5 && (
          <span
            style={{
              fontSize: 11, fontWeight: 700, color: "#9a3412",
              background: "#ffedd5", padding: "2px 8px", borderRadius: 999,
            }}
            title="This market's fair value sits in the 0.40-0.50 calibration bucket, where the market is historically least reliable - exactly why the Risk Officer scrutinises the agent's pick here"
          >
            calibration: caution
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
      <div style={{ marginTop: 2, fontSize: 12, color: "var(--text-muted)", textAlign: "right" }}>
        {new Date(market.expiry).toISOString().slice(0, 10)}
      </div>
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
        {atm !== undefined && (
          <span style={{ display: "block", marginBottom: 2 }}>ATM ${atm.toLocaleString(undefined, { maximumFractionDigits: 0 })}</span>
        )}
        {market.oracle_id.slice(0, 18)}…
      </div>
    </div>
    </Link>
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
