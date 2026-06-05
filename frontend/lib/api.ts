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


// --- Strike grid (single market detail) ---

export interface StrikeRow {
  strike_usd: number;
  log_moneyness: number;
  total_variance: number;
  implied_vol_annualized: number;
  fair_up: number;
  fair_down: number;
}

export interface StrikeGrid {
  spot_usd: number;
  forward_usd: number;
  seconds_until_expiry: number;
  atm_strike_usd: number;
  strikes: StrikeRow[];
}

export interface StrikesResponse {
  oracle: MarketSummary;
  grid: StrikeGrid;
}

// --- Edge grid (fair vs market) ---

export type EdgeSignal = "no_data" | "buy" | "avoid" | "neutral";

export interface DirectionalEdge {
  fair: number;
  market_ask: number | null;
  ev: number | null;
  signal: EdgeSignal;
  market_ts_ms: number | null;
}

export interface StrikeEdge {
  strike_usd: number;
  log_moneyness: number;
  implied_vol_annualized: number | null;
  up: DirectionalEdge;
  down: DirectionalEdge;
}

export interface BestEdge {
  strike_usd: number;
  direction: string;
  fair: number;
  market_ask: number;
  ev: number;
}

export interface EdgeGrid {
  spot_usd: number;
  forward_usd: number;
  seconds_until_expiry: number;
  atm_strike_usd: number;
  market_data_points: number;
  best_edge: BestEdge | null;
  strikes: StrikeEdge[];
}

export interface EdgesResponse {
  oracle: MarketSummary;
  edge_grid: EdgeGrid | null;
}


// --- PredictManager lookup ---

export interface ManagerEvent {
  manager_id: string;
  owner: string;
  digest: string;
  checkpoint_timestamp_ms: number;
}


export interface PositionMint {
  digest: string;
  oracle_id: string;
  is_up: boolean;
  strike: number;
  quantity: number;
  cost: number;
  ask_price: number;
  checkpoint_timestamp_ms: number;
}

export interface PositionsResponse {
  minted?: PositionMint[];
}

export interface ManagerSummary {
  open_positions: number;
  realized_pnl: number;
  account_value: number;
  balances: { quote_asset: string; balance: number }[];
}

export const api = {
  markets: () => getJson<MarketsResponse>("/api/markets"),
  calibration: () => getJson<CalibrationReport>("/api/backtest/calibration"),
  strikes: (oracleId: string) =>
    getJson<StrikesResponse>(`/api/markets/${oracleId}/strikes?num=15&step=50`),
  edges: (oracleId: string) =>
    getJson<EdgesResponse>(`/api/markets/${oracleId}/edges?num=15&step=50`),
  manager: (owner: string) =>
    getJson<ManagerEvent[]>(`/api/manager?owner=${owner}`),
  positions: (managerId: string) =>
    getJson<PositionsResponse>(`/api/manager/positions?manager=${managerId}`),
  summary: (managerId: string) =>
    getJson<ManagerSummary>(`/api/manager/summary?manager=${managerId}`),
};
