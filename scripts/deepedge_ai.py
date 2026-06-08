#!/usr/bin/env python3
"""DeepEdge AI reasoning layer (Phase 2).
Reads fair value + calibration from the DeepEdge backend, asks Claude to
produce a calibration-aware betting judgment, and prints structured JSON.
"""
import json, os, sys, urllib.request

API = 'http://localhost:3000'
ANTHROPIC_KEY = os.environ.get('ANTHROPIC_API_KEY')
MODEL = 'claude-sonnet-4-5-20250929'

def get(path):
    with urllib.request.urlopen(API + path, timeout=15) as r:
        return json.load(r)

def pick_active_market():
    mk = get('/api/markets')
    actives = [m for m in mk['markets'] if m.get('status') == 'active']
    if not actives:
        sys.exit('No active markets')
    return actives[0]['oracle_id']

def calibration_for_prob(cal, prob):
    """Find the calibration bucket covering this probability."""
    for b in cal['buckets']:
        if b['bucket_low'] <= prob < b['bucket_high']:
            return b
    return None

def build_prompt(oracle, atm, fair_up, fair_down, cal, bucket):
    """Assemble the reasoning prompt for Claude."""
    lines = []
    lines.append('You are a disciplined quant analyst for binary options on DeepBook Predict.')
    lines.append('Decide whether to bet on a short-dated BTC up/down market, using the')
    lines.append('model fair value AND the historical calibration of this system.')
    lines.append('')
    lines.append('MARKET:')
    lines.append(f"  underlying: {oracle['underlying_asset']}")
    lines.append(f"  expiry: {oracle['expiry_iso']}")
    lines.append(f"  ATM strike (USD): {atm}")
    lines.append(f"  model fair P(up):   {fair_up:.4f}")
    lines.append(f"  model fair P(down): {fair_down:.4f}")
    lines.append('  NOTE: no live order book on testnet, so there is no market price to')
    lines.append('  compare against. Judge the fair value itself, corrected by calibration.')
    lines.append('')
    lines.append('SYSTEM-WIDE CALIBRATION (past settled bets):')
    lines.append(f"  overall implied avg: {cal['overall_avg_implied']:.3f}")
    lines.append(f"  overall actual win rate: {cal['overall_win_rate']:.3f}")
    lines.append(f"  mean abs calibration error: {cal['mean_abs_calibration_error']:.3f}")
    lines.append('  => the model tends to be OPTIMISTIC: implied probabilities have')
    lines.append('     historically exceeded actual win rates.')
    if bucket:
        lines.append('')
        lines.append('CALIBRATION FOR THIS PROBABILITY BUCKET:')
        lines.append(f"  bucket: {bucket['bucket_low']:.2f}-{bucket['bucket_high']:.2f}")
        lines.append(f"  avg implied: {bucket['avg_implied_prob']:.3f}")
        lines.append(f"  actual win rate: {bucket['actual_win_rate']:.3f}")
        lines.append(f"  calibration gap: {bucket['calibration_gap']:.3f} (negative = optimistic)")
    lines.append('')
    lines.append('TASK: Respond with ONLY a JSON object, no prose, no markdown fences:')
    lines.append('{')
    lines.append('  "recommendation": "BET_UP" | "BET_DOWN" | "NO_BET",')
    lines.append('  "confidence": <float 0..1>,')
    lines.append('  "calibration_adjusted_prob": <float 0..1, the fair P corrected for the bias>,')
    lines.append('  "reasoning": "<two or three sentences>"')
    lines.append('}')
    return chr(10).join(lines)

def ask_claude(prompt):
    body = json.dumps({
        'model': MODEL,
        'max_tokens': 500,
        'messages': [{'role': 'user', 'content': prompt}],
    }).encode()
    req = urllib.request.Request(
        'https://api.anthropic.com/v1/messages', data=body,
        headers={
            'x-api-key': ANTHROPIC_KEY,
            'anthropic-version': '2023-06-01',
            'content-type': 'application/json',
        })
    with urllib.request.urlopen(req, timeout=60) as r:
        resp = json.load(r)
    return resp['content'][0]['text']

def main():
    if not ANTHROPIC_KEY:
        sys.exit('Set ANTHROPIC_API_KEY (source ~/.anthropic_key)')
    oid = sys.argv[1] if len(sys.argv) > 1 else pick_active_market()
    edges = get(f'/api/markets/{oid}/edges')
    oracle = edges['oracle']
    grid = edges['edge_grid']
    atm = grid['atm_strike_usd']
    # pick the strike closest to ATM
    strikes = grid['strikes']
    near = min(strikes, key=lambda s: abs(s['strike_usd'] - atm))
    fair_up = near['up']['fair']
    fair_down = near['down']['fair']
    cal = get('/api/backtest/calibration')
    bucket = calibration_for_prob(cal, fair_up)
    prompt = build_prompt(oracle, near['strike_usd'], fair_up, fair_down, cal, bucket)
    print('=== Market:', oracle['underlying_asset'], oracle['expiry_iso'], '===')
    print(f'  strike {near["strike_usd"]}, fair up {fair_up:.4f} / down {fair_down:.4f}')
    print('=== Asking Claude... ===')
    raw = ask_claude(prompt)
    # strip accidental fences
    raw = raw.strip().removeprefix('```json').removeprefix('```').removesuffix('```').strip()
    try:
        judgment = json.loads(raw)
        print(json.dumps(judgment, indent=2, ensure_ascii=False))
    except json.JSONDecodeError:
        print('Claude did not return clean JSON:')
        print(raw)

if __name__ == '__main__':
    main()
