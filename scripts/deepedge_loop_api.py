#!/usr/bin/env python3
"""DeepEdge two-agent autonomous loop (Strategist + Risk Officer).

A Strategist agent proposes a bet from the market data; a separate Risk
Officer agent reviews it against the historical calibration and the
Mandate limits, and can VETO. Only an approved decision is stored on
Walrus, hashed, enforced on-chain, and verified. Two AIs check each
other; the Move Mandate is the final hard rail.
"""
import json, os, sys, time, hashlib, subprocess, urllib.request

API = 'http://localhost:3000'
PKG = '0xb82750b35a213320d5ad6204e7bce46493ae76340e2a018fd65fdca4ad08f34a'
MANDATE = '0x753fb2e637d42067aeea59df6044ddfeb37ac22c92f28c89a8ffc6e3a4635f3a'
WAL_PUB = 'https://publisher.walrus-testnet.walrus.space/v1/blobs?epochs=1'
WAL_AGG = 'https://aggregator.walrus-testnet.walrus.space/v1/blobs/'
KEY = os.environ.get('ANTHROPIC_API_KEY')
MODEL = 'claude-sonnet-4-5-20250929'
PER_BET_CAP = 2000000
TOTAL_BUDGET = 10000000

def get(path):
    with urllib.request.urlopen(API + path, timeout=15) as r:
        return json.load(r)

def claude(system, user, max_tokens=600):
    body = json.dumps({
        'model': MODEL, 'max_tokens': max_tokens,
        'system': system,
        'messages': [{'role': 'user', 'content': user}],
    }).encode()
    req = urllib.request.Request('https://api.anthropic.com/v1/messages',
        data=body, headers={'x-api-key': KEY,
        'anthropic-version': '2023-06-01', 'content-type': 'application/json'})
    with urllib.request.urlopen(req, timeout=60) as r:
        txt = json.load(r)['content'][0]['text']
    return txt.strip().removeprefix('```json').removeprefix('```').removesuffix('```').strip()

# ---- observe ----
def observe(oid=None):
    if oid is None:
        mk = get('/api/markets')
        actives = [m for m in mk['markets'] if m.get('status') == 'active']
        if not actives:
            sys.exit('No active markets')
        oid = actives[0]['oracle_id']
    edges = get(f'/api/markets/{oid}/edges')
    cal = get('/api/backtest/calibration')
    grid = edges['edge_grid']
    atm = grid['atm_strike_usd']
    near = min(grid['strikes'], key=lambda s: abs(s['strike_usd'] - atm))
    return oid, edges['oracle'], near, cal

def bucket_for(cal, prob):
    for b in cal['buckets']:
        if b['bucket_low'] <= prob < b['bucket_high']:
            return b
    return None

# ---- Agent 1: Strategist ----
def strategist(oracle, near, cal):
    fair_up = near['up']['fair']
    fair_down = near['down']['fair']
    sys_p = ('You are the STRATEGIST for a DeepBook Predict trading agent. '
             'You propose bets to maximize expected value. Be decisive but '
             'honest. Respond ONLY with JSON, no fences.')
    u = []
    u.append(f"Market {oracle['underlying_asset']} expiry {oracle['expiry_iso']}")
    u.append(f"strike {near['strike_usd']}, model fair P(up) {fair_up:.4f}, P(down) {fair_down:.4f}")
    u.append(f"per-bet cap {PER_BET_CAP} (1e6=1 DUSDC), budget {TOTAL_BUDGET}.")
    u.append('Propose a bet. JSON:')
    u.append('{"action":"BET_UP|BET_DOWN|NO_BET","size":<int micro-DUSDC <= cap>,"thesis":"<why, 2 sentences>"}')
    return json.loads(claude(sys_p, chr(10).join(u)))

# ---- Agent 2: Risk Officer ----
def risk_officer(proposal, oracle, near, cal):
    fair_up = near['up']['fair']
    b = bucket_for(cal, fair_up)
    sys_p = ('You are the RISK OFFICER for a DeepBook Predict trading agent. '
             'Your job is to PROTECT capital. Review the Strategist proposal '
             'against historical calibration (the model is systematically '
             'optimistic) and the mandate limits. You may VETO or cut size. '
             'Respond ONLY with JSON, no fences.')
    u = []
    u.append('STRATEGIST PROPOSAL: ' + json.dumps(proposal))
    u.append(f"model fair P(up) {fair_up:.4f}")
    u.append(f"CALIBRATION overall: implied {cal['overall_avg_implied']:.3f} vs actual {cal['overall_win_rate']:.3f}, ROI {cal['overall_avg_roi']:.3f}")
    if b:
        u.append(f"this bucket {b['bucket_low']:.2f}-{b['bucket_high']:.2f}: implied {b['avg_implied_prob']:.3f} actual {b['actual_win_rate']:.3f} gap {b['calibration_gap']:.3f} roi {b['avg_roi']:.3f}")
    u.append(f"limits: per-bet cap {PER_BET_CAP}, budget {TOTAL_BUDGET}.")
    u.append('Review. JSON:')
    u.append('{"approved":true|false,"adjusted_size":<int micro-DUSDC, 0 if vetoed>,"calibration_adjusted_prob":<0..1>,"verdict":"<reasoning, 2-3 sentences>"}')
    return json.loads(claude(sys_p, chr(10).join(u)))

# ---- Walrus + verify ----
def store_walrus(b):
    req = urllib.request.Request(WAL_PUB, data=b, method='PUT')
    with urllib.request.urlopen(req, timeout=60) as r:
        resp = json.load(r)
    return (resp.get('newlyCreated', {}).get('blobObject', {}).get('blobId')
            or resp.get('alreadyCertified', {}).get('blobId'))

def verify(blob_id, expected_hex, retries=6, wait=5):
    last = None
    for i in range(retries):
        try:
            req = urllib.request.Request(WAL_AGG + blob_id,
                headers={'User-Agent': 'deepedge-loop2/1.0'})
            with urllib.request.urlopen(req, timeout=30) as r:
                data = r.read()
            return hashlib.sha256(data).hexdigest() == expected_hex
        except urllib.error.HTTPError as e:
            last = e
            if e.code in (403, 404, 425):
                print(f'    (aggregator not ready, retry {i+1}/{retries})')
                time.sleep(wait); continue
            raise
    print(f'    verify gave up: {last}')
    return False

def enforce_and_record(amount, hash_hex, blob_id):
    cmd = ['sui', 'client', 'ptb',
        '--move-call', f'{PKG}::mandate::authorize_with_decision',
        f'@{MANDATE}', f'{amount}', f'"{hash_hex}"', f'"{blob_id}"',
        '--assign', 'receipt',
        '--move-call', f'{PKG}::mandate::record_decision_and_consume',
        f'@{MANDATE}', 'receipt', '--gas-budget', '50000000']
    return subprocess.run(cmd, capture_output=True, text=True)

def run_cycle_json():
    """Run one 2-agent cycle and return a structured dict (for the API)."""
    import io, contextlib
    steps = []
    result = {"ok": False, "steps": steps}
    try:
        # 1. observe
        oid, oracle, near, cal = observe(None)
        steps.append({"stage": "observe", "status": "done",
            "market": {"asset": oracle["underlying_asset"],
                       "expiry": oracle["expiry_iso"],
                       "strike_usd": near["strike_usd"],
                       "oracle_id": oid},
            "fair": {"up": near["up"]["fair"], "down": near["down"]["fair"]}})

        # 2. strategist
        prop = strategist(oracle, near, cal)
        steps.append({"stage": "strategist", "status": "done", "proposal": prop})

        # 3. risk officer
        review = risk_officer(prop, oracle, near, cal)
        steps.append({"stage": "risk_officer", "status": "done", "review": review})

        approved = bool(review.get("approved"))
        size = int(review.get("adjusted_size", 0)) if approved else 0
        if size > PER_BET_CAP:
            size = PER_BET_CAP

        # 4. decision record + walrus
        record = {
            "market": {"oracle_id": oid, "asset": oracle["underlying_asset"],
                       "expiry": oracle["expiry_iso"], "strike_usd": near["strike_usd"]},
            "fair": {"up": near["up"]["fair"], "down": near["down"]["fair"]},
            "strategist": prop, "risk_officer": review,
            "final_size": size, "model": MODEL, "ts": int(time.time())}
        rb = json.dumps(record, sort_keys=True, separators=(",", ":")).encode()
        hash_hex = hashlib.sha256(rb).hexdigest()
        blob_id = store_walrus(rb)
        steps.append({"stage": "walrus", "status": "done",
            "blob_id": blob_id, "sha256": hash_hex})

        # 5. verify
        ok = verify(blob_id, hash_hex)
        steps.append({"stage": "verify", "status": "done", "match": ok})

        # 6. enforce + record (only if approved)
        if approved and size > 0:
            res = enforce_and_record(size, hash_hex, blob_id)
            if res.returncode == 0:
                dg = [l.strip() for l in res.stdout.splitlines() if "Transaction Digest" in l]
                digest = dg[0].split(":")[-1].strip() if dg else ""
                steps.append({"stage": "enforce", "status": "done",
                    "spent_amount": size, "digest": digest, "vetoed": False})
            else:
                steps.append({"stage": "enforce", "status": "error",
                    "error": res.stderr[-300:]})
        else:
            steps.append({"stage": "enforce", "status": "vetoed",
                "vetoed": True,
                "reason": review.get("verdict", "Risk Officer vetoed")})

        result["ok"] = True
        result["approved"] = approved
        result["final_size"] = size
    except Exception as e:
        result["error"] = str(e)
    return result


def main():
    if not KEY:
        print(json.dumps({"ok": False, "error": "ANTHROPIC_API_KEY not set"}))
        return
    if len(sys.argv) > 1 and sys.argv[1] == "--json":
        print(json.dumps(run_cycle_json()))
        return
    # original human-readable mode preserved below
    _main_human()


def _main_human():
    if not KEY:
        sys.exit('source ~/.anthropic_key first')
    oid_arg = sys.argv[1] if len(sys.argv) > 1 else None

    print('== 1. OBSERVE ==')
    oid, oracle, near, cal = observe(oid_arg)
    print(f"  {oracle['underlying_asset']} {oracle['expiry_iso']} strike {near['strike_usd']}")

    print('== 2. STRATEGIST proposes ==')
    prop = strategist(oracle, near, cal)
    print('  ' + json.dumps(prop, ensure_ascii=False))

    print('== 3. RISK OFFICER reviews ==')
    review = risk_officer(prop, oracle, near, cal)
    print('  ' + json.dumps(review, ensure_ascii=False))

    approved = bool(review.get('approved'))
    size = int(review.get('adjusted_size', 0)) if approved else 0
    # safety clamp to the on-chain cap
    if size > PER_BET_CAP:
        size = PER_BET_CAP

    print('== 4. DECISION RECORD + WALRUS ==')
    record = {
        'market': {'oracle_id': oid, 'asset': oracle['underlying_asset'],
                   'expiry': oracle['expiry_iso'], 'strike_usd': near['strike_usd']},
        'fair': {'up': near['up']['fair'], 'down': near['down']['fair']},
        'strategist': prop,
        'risk_officer': review,
        'final_size': size,
        'model': MODEL, 'ts': int(time.time()),
    }
    rb = json.dumps(record, sort_keys=True, separators=(',', ':')).encode()
    hash_hex = hashlib.sha256(rb).hexdigest()
    blob_id = store_walrus(rb)
    print(f'  sha256 {hash_hex}')
    print(f'  blobId {blob_id}')

    print('== 5. VERIFY ==')
    print(f"  hashes back: {verify(blob_id, hash_hex)}")

    print('== 6. ENFORCE + RECORD ==')
    if approved and size > 0:
        res = enforce_and_record(size, hash_hex, blob_id)
        if res.returncode == 0:
            dg = [l.strip() for l in res.stdout.splitlines() if 'Transaction Digest' in l]
            print(f'  recorded on-chain size={size}; ' + (dg[0] if dg else ''))
        else:
            print('  FAILED:'); print(res.stderr[-400:])
    else:
        print(f'  Risk Officer VETOED (or NO_BET). Reasoning stored on Walrus,')
        print(f'  no on-chain spend. This is the two-agent check working.')
    print('== CYCLE COMPLETE ==')

if __name__ == '__main__':
    main()
