// DeepEdge backend client (browser-side fetch, CORS enabled)

const API_BASE =
  process.env.NEXT_PUBLIC_API_BASE || "http://localhost:3000";

export interface MarketSummary {
  oracle_id: string;
  underlying_asset: string;
  status: string;
  expiry: number;
  expiry_iso: string;
  min_strike_usd: number;
  tick_size_usd: number;
  settlement_price_usd: number | null;
  created_checkpoint: number;
}

export interface MarketsResponse {
  count: number;
  markets: MarketSummary[];
}

export interface CalibrationBucket {
  bucket_low: number;
  bucket_high: number;
  bet_count: number;
  avg_implied_prob: number;
  actual_win_rate: number;
  calibration_gap: number;
  avg_roi: number;
  up_count: number;
  up_implied: number;
  up_actual: number;
  down_count: number;
  down_implied: number;
  down_actual: number;
}

export interface CalibrationReport {
  total_settled_oracles: number;
  total_bets_evaluated: number;
  overall_win_rate: number;
  overall_avg_implied: number;
  overall_avg_roi: number;
  mean_abs_calibration_error: number;
  buckets: CalibrationBucket[];
}

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, { cache: "no-store" });
  if (!res.ok) {
    throw new Error(`API ${path} failed: ${res.status}`);
  }
  return res.json();
}

export const api = {
  markets: () => getJson<MarketsResponse>("/api/markets"),
  calibration: () => getJson<CalibrationReport>("/api/backtest/calibration"),
};
