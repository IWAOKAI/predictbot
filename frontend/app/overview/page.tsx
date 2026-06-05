"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { api, MarketSummary, StrikesResponse } from "@/lib/api";

interface Row {
  market: MarketSummary;
  spot: number;
  atmFairUp: number;
  atmIv: number;
  minsLeft: number;
}

export default function OverviewPage() {
  const [rows, setRows] = useState<Row[] | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let active = true;
    api
      .markets()
      .then(async (res) => {
        const activeMarkets = res.markets
          .filter((m) => m.status === "active")
          .sort((a, b) => a.expiry - b.expiry);

        const settled = await Promise.allSettled(
          activeMarkets.map((m) => api.strikes(m.oracle_id))
        );

        const built: Row[] = [];
        settled.forEach((r, i) => {
          if (r.status === "fulfilled") {
            const s: StrikesResponse = r.value;
            const atm = s.grid.strikes.reduce((best, row) =>
              Math.abs(row.strike_usd - s.grid.atm_strike_usd) <
              Math.abs(best.strike_usd - s.grid.atm_strike_usd)
                ? row
                : best
            , s.grid.strikes[0]);
            built.push({
              market: activeMarkets[i],
              spot: s.grid.spot_usd,
              atmFairUp: atm.fair_up,
              atmIv: atm.implied_vol_annualized,
              minsLeft: Math.floor(s.grid.seconds_until_expiry / 60),
            });
          }
        });

        if (active) setRows(built);
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, []);

  return (
    <div>
      <div style={{ marginBottom: 24 }}>
        <h1
          style={{
            fontSize: 28,
            fontWeight: 800,
            margin: "0 0 6px",
            color: "var(--primary-dark)",
          }}
        >
          Market Overview
        </h1>
        <p style={{ color: "var(--text-muted)", margin: 0, fontSize: 14 }}>
          DeepEdge fair value across every live BTC market, sorted by expiry.
          One glance at the whole board.
        </p>
      </div>

      {loading || !rows ? (
        <div className="card" style={{ padding: 32, textAlign: "center", color: "var(--text-muted)" }}>
          Pricing all live markets...
        </div>
      ) : (
        <div className="card" style={{ padding: 0, overflowX: "auto" }}>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 13 }}>
            <thead>
              <tr style={{ textAlign: "right", color: "var(--text-muted)", background: "#f8fafc" }}>
                <th style={{ padding: "12px 14px", textAlign: "left" }}>Market</th>
                <th style={{ padding: "12px 14px" }}>Expires</th>
                <th style={{ padding: "12px 14px" }}>Spot</th>
                <th style={{ padding: "12px 14px" }}>Fair UP (ATM)</th>
                <th style={{ padding: "12px 14px" }}>Fair DOWN</th>
                <th style={{ padding: "12px 14px" }}>IV</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => {
                const soon = r.minsLeft < 5;
                const timeStr =
                  r.minsLeft < 60
                    ? `${r.minsLeft}m`
                    : `${Math.floor(r.minsLeft / 60)}h ${r.minsLeft % 60}m`;
                return (
                  <tr
                    key={r.market.oracle_id}
                    style={{ borderTop: "1px solid var(--border)" }}
                  >
                    <td style={{ padding: "10px 14px", textAlign: "left" }}>
                      <Link
                        href={`/market/${r.market.oracle_id}`}
                        style={{ color: "var(--primary)", textDecoration: "none", fontWeight: 600 }}
                      >
                        {r.market.underlying_asset}
                      </Link>
                    </td>
                    <td
                      style={{
                        padding: "10px 14px",
                        textAlign: "right",
                        color: soon ? "var(--down)" : "var(--text)",
                        fontWeight: soon ? 700 : 400,
                      }}
                    >
                      {timeStr}
                    </td>
                    <td style={{ padding: "10px 14px", textAlign: "right" }}>
                      ${r.spot.toLocaleString(undefined, { maximumFractionDigits: 0 })}
                    </td>
                    <td style={{ padding: "10px 14px", textAlign: "right", color: "var(--up)", fontWeight: 600 }}>
                      {(r.atmFairUp * 100).toFixed(1)}%
                    </td>
                    <td style={{ padding: "10px 14px", textAlign: "right", color: "var(--down)" }}>
                      {((1 - r.atmFairUp) * 100).toFixed(1)}%
                    </td>
                    <td style={{ padding: "10px 14px", textAlign: "right", color: "var(--text-muted)" }}>
                      {(r.atmIv * 100).toFixed(0)}%
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {rows && (
        <p style={{ fontSize: 12, color: "var(--text-muted)", marginTop: 12 }}>
          Showing {rows.length} live markets. Fair UP is DeepEdge&apos;s model
          probability BTC settles above the at-the-money strike.
        </p>
      )}
    </div>
  );
}
