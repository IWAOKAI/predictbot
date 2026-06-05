"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import Link from "next/link";
import { api, StrikesResponse, EdgesResponse, StrikeRow } from "@/lib/api";
import { BetPanel } from "@/components/BetPanel";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  ReferenceLine,
} from "recharts";

export default function MarketDetailPage() {
  const params = useParams();
  const oracleId = params.oracle_id as string;

  const [data, setData] = useState<StrikesResponse | null>(null);
  const [edges, setEdges] = useState<EdgesResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([api.strikes(oracleId), api.edges(oracleId)])
      .then(([s, e]) => {
        setData(s);
        setEdges(e);
      })
      .catch((err) => setError(err.message))
      .finally(() => setLoading(false));
  }, [oracleId]);

  if (loading)
    return (
      <div style={{ padding: 48, textAlign: "center", color: "var(--text-muted)" }}>
        Loading market…
      </div>
    );
  if (error || !data)
    return (
      <div className="card" style={{ padding: 24, color: "var(--down)" }}>
        Could not load market: {error}
      </div>
    );

  const { oracle, grid } = data;
  const mins = Math.max(0, Math.floor(grid.seconds_until_expiry / 60));
  const timeStr = mins < 60 ? `${mins}m` : `${Math.floor(mins / 60)}h ${mins % 60}m`;

  // SVI smile chart data: strike vs implied vol (%)
  const smileData = grid.strikes.map((s) => ({
    strike: Math.round(s.strike_usd),
    iv: +(s.implied_vol_annualized * 100).toFixed(1),
    fairUp: +(s.fair_up * 100).toFixed(1),
  }));

  const marketPoints = edges?.edge_grid?.market_data_points ?? 0;

  return (
    <div>
      <Link
        href="/"
        style={{
          fontSize: 13,
          color: "var(--primary)",
          textDecoration: "none",
          fontWeight: 600,
        }}
      >
        ← All markets
      </Link>

      <div style={{ margin: "12px 0 24px" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <h1
            style={{
              fontSize: 28,
              fontWeight: 800,
              margin: 0,
              color: "var(--primary-dark)",
            }}
          >
            {oracle.underlying_asset} market
          </h1>
          <span
            className={
              oracle.status === "active" ? "badge badge-active" : "badge badge-settled"
            }
          >
            {oracle.status}
          </span>
        </div>
        <div
          style={{
            fontSize: 12,
            color: "var(--text-muted)",
            fontFamily: "ui-monospace, monospace",
            marginTop: 4,
          }}
        >
          {oracle.oracle_id}
        </div>
      </div>

      {/* price stats */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(150px, 1fr))",
          gap: 14,
          marginBottom: 24,
        }}
      >
        <Stat label="Spot" value={`$${grid.spot_usd.toLocaleString(undefined, { maximumFractionDigits: 0 })}`} />
        <Stat label="Forward" value={`$${grid.forward_usd.toLocaleString(undefined, { maximumFractionDigits: 0 })}`} />
        <Stat label="ATM strike" value={`$${grid.atm_strike_usd.toLocaleString()}`} />
        <Stat label="Expires in" value={timeStr} />
      </div>

      {/* SVI smile */}
      <div className="card" style={{ padding: 24, marginBottom: 24 }}>
        <h2 style={{ fontSize: 17, fontWeight: 700, margin: "0 0 4px" }}>
          Volatility smile (SVI)
        </h2>
        <p style={{ fontSize: 13, color: "var(--text-muted)", margin: "0 0 18px" }}>
          Annualized implied volatility across strikes, from the on-chain SVI
          parameters.
        </p>
        <ResponsiveContainer width="100%" height={280}>
          <LineChart data={smileData} margin={{ top: 8, right: 12, bottom: 8, left: 0 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="#e0f2fe" />
            <XAxis
              dataKey="strike"
              tick={{ fontSize: 11, fill: "#64748b" }}
              tickFormatter={(v) => `$${(v / 1000).toFixed(1)}k`}
            />
            <YAxis tick={{ fontSize: 11, fill: "#64748b" }} domain={["auto", "auto"]} />
            <Tooltip
              contentStyle={{ borderRadius: 12, border: "1px solid var(--border)", fontSize: 13 }}
              formatter={(v) => [`${Number(v)}%`, "IV"]}
              labelFormatter={(v) => `Strike $${Number(v).toLocaleString()}`}
            />
            <ReferenceLine
              x={Math.round(grid.atm_strike_usd)}
              stroke="#0ea5e9"
              strokeDasharray="4 4"
              label={{ value: "ATM", fontSize: 11, fill: "#0369a1" }}
            />
            <Line
              type="monotone"
              dataKey="iv"
              stroke="#0ea5e9"
              strokeWidth={2.5}
              dot={{ r: 3, fill: "#0ea5e9" }}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>

      {/* bet panel */}
      <div style={{ marginBottom: 24 }}>
        <BetPanel
          oracleId={oracle.oracle_id}
          expiry={oracle.expiry}
          atmStrike={grid.atm_strike_usd}
          strikes={grid.strikes.map((s) => ({
            strike_usd: s.strike_usd,
            fair_up: s.fair_up,
            fair_down: s.fair_down,
          }))}
        />
      </div>

      {/* fair probability table */}
      <div className="card" style={{ padding: 24, marginBottom: 24, overflowX: "auto" }}>
        <h2 style={{ fontSize: 17, fontWeight: 700, margin: "0 0 4px" }}>
          Fair probabilities by strike
        </h2>
        <p style={{ fontSize: 13, color: "var(--text-muted)", margin: "0 0 16px" }}>
          DeepEdge&apos;s model probability that BTC settles above (UP) or below
          (DOWN) each strike.
          {marketPoints > 0
            ? ` Market data points: ${marketPoints}.`
            : " No live market quotes for this oracle yet — showing fair value only."}
        </p>
        <StrikeTable strikes={grid.strikes} atm={grid.atm_strike_usd} />
      </div>
    </div>
  );
}

function StrikeTable({ strikes, atm }: { strikes: StrikeRow[]; atm: number }) {
  return (
    <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 13 }}>
      <thead>
        <tr style={{ textAlign: "right", color: "var(--text-muted)" }}>
          <th style={{ padding: "8px 10px", textAlign: "left" }}>Strike</th>
          <th style={{ padding: "8px 10px" }}>Fair UP</th>
          <th style={{ padding: "8px 10px" }}>Fair DOWN</th>
          <th style={{ padding: "8px 10px" }}>IV</th>
        </tr>
      </thead>
      <tbody>
        {strikes.map((s) => {
          const isAtm = Math.abs(s.strike_usd - atm) < 1;
          return (
            <tr
              key={s.strike_usd}
              style={{
                borderTop: "1px solid var(--border)",
                background: isAtm ? "#f0f9ff" : "transparent",
                fontWeight: isAtm ? 700 : 400,
              }}
            >
              <td style={{ padding: "8px 10px", textAlign: "left" }}>
                ${s.strike_usd.toLocaleString()}
                {isAtm && (
                  <span
                    style={{ color: "var(--primary)", fontSize: 11, marginLeft: 6 }}
                    title="At-the-money: strike closest to current price"
                  >
                    at current price
                  </span>
                )}
              </td>
              <td style={{ padding: "8px 10px", textAlign: "right", color: "var(--up)" }}>
                {(s.fair_up * 100).toFixed(1)}%
              </td>
              <td style={{ padding: "8px 10px", textAlign: "right", color: "var(--down)" }}>
                {(s.fair_down * 100).toFixed(1)}%
              </td>
              <td style={{ padding: "8px 10px", textAlign: "right", color: "var(--text-muted)" }}>
                {(s.implied_vol_annualized * 100).toFixed(0)}%
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="card" style={{ padding: 16 }}>
      <div style={{ fontSize: 12, color: "var(--text-muted)", fontWeight: 600 }}>
        {label}
      </div>
      <div style={{ fontSize: 22, fontWeight: 800, marginTop: 4, color: "var(--primary-dark)" }}>
        {value}
      </div>
    </div>
  );
}
