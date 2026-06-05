"use client";

import React, { useEffect, useState } from "react";
import {
  useCurrentAccount,
  useSignAndExecuteTransaction,
} from "@mysten/dapp-kit";
import { api, ManagerEvent } from "@/lib/api";
import { buildDepositTx, buildMintTx } from "@/lib/transactions";

interface BetPanelProps {
  oracleId: string;
  expiry: number;
  atmStrike: number;
  strikes: { strike_usd: number; fair_up: number; fair_down: number }[];
}

type Status =
  | { kind: "idle" }
  | { kind: "working"; msg: string }
  | { kind: "ok"; msg: string; digest?: string }
  | { kind: "error"; msg: string };

export function BetPanel({ oracleId, expiry, atmStrike, strikes }: BetPanelProps) {
  const account = useCurrentAccount();
  const { mutateAsync: signAndExecute } = useSignAndExecuteTransaction();

  const [managerId, setManagerId] = useState<string | null>(null);
  const [managerChecked, setManagerChecked] = useState(false);
  const [mgrBalance, setMgrBalance] = useState<number | null>(null);

  const [selectedStrike, setSelectedStrike] = useState<number>(atmStrike);
  const [isUp, setIsUp] = useState(true);
  const [betUsd, setBetUsd] = useState("5");
  const [depositUsd, setDepositUsd] = useState("10");
  const [status, setStatus] = useState<Status>({ kind: "idle" });

  async function refreshBalance(mid: string) {
    try {
      const summary = await fetch(
        `https://predict-server.testnet.mystenlabs.com/managers/${mid}/summary`
      ).then((r) => r.json());
      const dusdc = summary?.balances?.find(
        (entry: { quote_asset?: string; balance?: number }) =>
          entry.quote_asset?.includes("dusdc")
      );
      setMgrBalance((dusdc?.balance ?? 0) / 1e6);
    } catch {
      setMgrBalance(null);
    }
  }

  useEffect(() => {
    if (!account) {
      setManagerChecked(false);
      setManagerId(null);
      return;
    }
    api
      .manager(account.address)
      .then(async (events: ManagerEvent[]) => {
        if (events.length > 0) {
          setManagerId(events[0].manager_id);
          await refreshBalance(events[0].manager_id);
        } else {
          setManagerId(null);
        }
      })
      .catch(() => setManagerId(null))
      .finally(() => setManagerChecked(true));
  }, [account]);

  async function handleDeposit() {
    const amount = BigInt(Math.round(parseFloat(depositUsd) * 1e6));
    if (amount <= 0n || !managerId) return;
    setStatus({ kind: "working", msg: "Depositing DUSDC..." });
    try {
      const tx = buildDepositTx({ managerId, amount });
      const result = await signAndExecute({ transaction: tx });
      setStatus({ kind: "ok", msg: `Deposited $${depositUsd}`, digest: result.digest });
      await refreshBalance(managerId);
    } catch (e) {
      setStatus({ kind: "error", msg: (e as Error).message ?? "Deposit failed" });
    }
  }

  async function handleBet() {
    const quantity = BigInt(Math.round(parseFloat(betUsd) * 1e6));
    if (quantity <= 0n || !managerId) return;
    setStatus({ kind: "working", msg: "Placing bet..." });
    try {
      const tx = buildMintTx({
        managerId,
        oracleId,
        expiry: BigInt(expiry),
        strike: BigInt(Math.round(selectedStrike * 1e9)),
        isUp,
        quantity,
      });
      const result = await signAndExecute({ transaction: tx });
      setStatus({
        kind: "ok",
        msg: `Bet placed: ${isUp ? "UP" : "DOWN"} $${selectedStrike.toLocaleString()}`,
        digest: result.digest,
      });
      await refreshBalance(managerId);
    } catch (e) {
      setStatus({ kind: "error", msg: (e as Error).message ?? "Bet failed" });
    }
  }

  if (!account) {
    return <Panel><Muted>Connect your wallet to bet on this market.</Muted></Panel>;
  }
  if (!managerChecked) {
    return <Panel><Muted>Checking your PredictManager...</Muted></Panel>;
  }
  if (!managerId) {
    return (
      <Panel>
        <p style={{ margin: "0 0 8px", fontWeight: 600 }}>No PredictManager found</p>
        <Muted>You need a PredictManager to bet. Create one via the DeepBook Predict app for now.</Muted>
      </Panel>
    );
  }

  const selectedRow = strikes.find((s) => Math.abs(s.strike_usd - selectedStrike) < 1);
  const fairPct = selectedRow
    ? ((isUp ? selectedRow.fair_up : selectedRow.fair_down) * 100).toFixed(1)
    : "-";

  return (
    <Panel>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 14 }}>
        <h2 style={{ fontSize: 17, fontWeight: 700, margin: 0 }}>Place a bet</h2>
        <span style={{ fontSize: 12, color: "var(--text-muted)" }}>
          Manager balance:{" "}
          <strong style={{ color: "var(--primary-dark)" }}>
            {mgrBalance === null ? "..." : `$${mgrBalance.toFixed(2)}`}
          </strong>
        </span>
      </div>

      <div style={{ marginBottom: 18, paddingBottom: 18, borderBottom: "1px solid var(--border)" }}>
        <label style={{ fontSize: 13, color: "var(--text-muted)" }}>Deposit DUSDC into your manager</label>
        <div style={{ display: "flex", gap: 8, marginTop: 6 }}>
          <input type="number" value={depositUsd} onChange={(e) => setDepositUsd(e.target.value)} style={inputStyle} />
          <button onClick={handleDeposit} style={secondaryBtn}>Deposit</button>
        </div>
      </div>

      <div style={{ display: "flex", gap: 8, marginBottom: 14 }}>
        <button onClick={() => setIsUp(true)} style={isUp ? upBtnActive : upBtn}>UP</button>
        <button onClick={() => setIsUp(false)} style={!isUp ? downBtnActive : downBtn}>DOWN</button>
      </div>

      <label style={{ fontSize: 13, color: "var(--text-muted)" }}>Strike</label>
      <select
        value={selectedStrike}
        onChange={(e) => setSelectedStrike(parseFloat(e.target.value))}
        style={{ ...inputStyle, width: "100%", marginTop: 6, marginBottom: 14 }}
      >
        {strikes.map((s) => (
          <option key={s.strike_usd} value={s.strike_usd}>
            ${s.strike_usd.toLocaleString()}{Math.abs(s.strike_usd - atmStrike) < 1 ? " (ATM)" : ""}
          </option>
        ))}
      </select>

      <div style={{ fontSize: 13, color: "var(--text-muted)", marginBottom: 14 }}>
        DeepEdge fair probability:{" "}
        <strong style={{ color: isUp ? "var(--up)" : "var(--down)" }}>{fairPct}%</strong>
      </div>

      <label style={{ fontSize: 13, color: "var(--text-muted)" }}>Bet amount (DUSDC)</label>
      <div style={{ display: "flex", gap: 8, marginTop: 6 }}>
        <input type="number" value={betUsd} onChange={(e) => setBetUsd(e.target.value)} style={inputStyle} />
        <button onClick={handleBet} style={primaryBtn}>Bet {isUp ? "UP" : "DOWN"}</button>
      </div>

      {status.kind !== "idle" && (
        <div
          style={{
            marginTop: 14,
            padding: 12,
            borderRadius: 10,
            fontSize: 13,
            background: status.kind === "error" ? "#fef2f2" : status.kind === "ok" ? "#f0fdf4" : "#f0f9ff",
            color: status.kind === "error" ? "var(--down)" : status.kind === "ok" ? "#15803d" : "var(--primary-dark)",
          }}
        >
          {status.kind === "working" && status.msg}
          {status.kind === "error" && `x ${status.msg}`}
          {status.kind === "ok" && (
            <>
              {status.msg}
              {status.digest && (
                <a
                  href={`https://testnet.suivision.xyz/txblock/${status.digest}`}
                  target="_blank"
                  rel="noreferrer"
                  style={{ display: "block", marginTop: 4, color: "var(--primary)", fontSize: 12 }}
                >
                  View transaction
                </a>
              )}
            </>
          )}
        </div>
      )}
    </Panel>
  );
}

function Panel({ children }: { children: React.ReactNode }) {
  return <div className="card" style={{ padding: 24 }}>{children}</div>;
}

function Muted({ children }: { children: React.ReactNode }) {
  return <p style={{ color: "var(--text-muted)", margin: 0 }}>{children}</p>;
}

const inputStyle: React.CSSProperties = {
  flex: 1,
  padding: "8px 12px",
  borderRadius: 10,
  border: "1px solid var(--border)",
  fontSize: 14,
  color: "var(--text)",
};
const primaryBtn: React.CSSProperties = {
  padding: "8px 20px",
  borderRadius: 10,
  border: "none",
  background: "var(--primary)",
  color: "white",
  fontWeight: 700,
  cursor: "pointer",
};
const secondaryBtn: React.CSSProperties = {
  padding: "8px 20px",
  borderRadius: 10,
  border: "1px solid var(--primary)",
  background: "white",
  color: "var(--primary)",
  fontWeight: 600,
  cursor: "pointer",
};
const upBtn: React.CSSProperties = {
  flex: 1,
  padding: "10px",
  borderRadius: 10,
  border: "1px solid var(--border)",
  background: "white",
  color: "var(--up)",
  fontWeight: 700,
  cursor: "pointer",
};
const upBtnActive: React.CSSProperties = { ...upBtn, background: "var(--up)", color: "white", border: "none" };
const downBtn: React.CSSProperties = { ...upBtn, color: "var(--down)" };
const downBtnActive: React.CSSProperties = { ...downBtn, background: "var(--down)", color: "white", border: "none" };
