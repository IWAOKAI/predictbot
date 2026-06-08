#!/usr/bin/env python3
"""DeepEdge autonomous loop (Phase 3, full cycle).

observe -> reason (Claude) -> store reasoning on Walrus -> hash it ->
enforce + record on-chain via the Mandate contract -> verify the blob
hashes back to what is on chain. The Mandate's per-bet cap / budget /
kill switch are the safety rails that make an autonomous agent safe.
"""
import json, os, sys, time, hashlib, subprocess, urllib.request

API = 'http://localhost:3000'
PKG = '0xb82750b35a213320d5ad6204e7bce46493ae76340e2a018fd65fdca4ad08f34a'
MANDATE = '0x753fb2e637d42067aeea59df6044ddfeb37ac22c92f28c89a8ffc6e3a4635f3a'
WAL_PUB = 'https://publisher.walrus-testnet.walrus.space/v1/blobs?epochs=1'
WAL_AGG = 'https://aggregator.walrus-testnet.walrus.space/v1/blobs/'
ANTHROPIC_KEY = os.environ.get('ANTHROPIC_API_KEY')
MODEL = 'claude-sonnet-4-5-20250929'

def get(path):
    with urllib.request.urlopen(API + path, timeout=15) as r:
        return json.load(r)

# ---- 1. observe ----
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

# ---- 2. reason (Claude) ----
def reason(oracle, near, cal):
    fair_up = near['up']['fair']
    fair_down = near['down']['fair']
    b = bucket_for(cal, fair_up)
    p = []
    p.append('You are a disciplined quant analyst for DeepBook Predict binary options.')
    p.append('Use the model fair value AND historical calibration. No live book on testnet.')
    p.append('')
    p.append(f"MARKET: {oracle['underlying_asset']} expiry {oracle['expiry_iso']}")
    p.append(f"  strike {near['strike_usd']}, fair P(up) {fair_up:.4f}, P(down) {fair_down:.4f}")
    p.append(f"CALIBRATION: overall implied {cal['overall_avg_implied']:.3f} vs actual {cal['overall_win_rate']:.3f}")
    p.append(f"  mean abs calibration error {cal['mean_abs_calibration_error']:.3f} (model is optimistic)")
    if b:
        p.append(f"  this bucket {b['bucket_low']:.2f}-{b['bucket_high']:.2f}: implied {b['avg_implied_prob']:.3f} actual {b['actual_win_rate']:.3f} gap {b['calibration_gap']:.3f}")
    p.append('')
    p.append('Respond with ONLY JSON, no fences:')
    p.append('{"recommendation":"BET_UP|BET_DOWN|NO_BET","confidence":<0..1>,"calibration_adjusted_prob":<0..1>,"reasoning":"<2-3 sentences>"}')
    prompt = chr(10).join(p)
    body = json.dumps({'model': MODEL, 'max_tokens': 500,
                       'messages': [{'role': 'user', 'content': prompt}]}).encode()
    req = urllib.request.Request('https://api.anthropic.com/v1/messages', data=body,
        headers={'x-api-key': ANTHROPIC_KEY, 'anthropic-version': '2023-06-01',
                 'content-type': 'application/json'})
    with urllib.request.urlopen(req, timeout=60) as r:
        txt = json.load(r)['content'][0]['text']
    txt = txt.strip().removeprefix('```json').removeprefix('```').removesuffix('```').strip()
    return json.loads(txt), {'fair_up': fair_up, 'fair_down': fair_down, 'bucket': b}

# ---- 3. store on Walrus ----
def store_walrus(record_bytes):
    req = urllib.request.Request(WAL_PUB, data=record_bytes, method='PUT')
    with urllib.request.urlopen(req, timeout=60) as r:
        resp = json.load(r)
    return (resp.get('newlyCreated', {}).get('blobObject', {}).get('blobId')
            or resp.get('alreadyCertified', {}).get('blobId'))

# ---- 4. verify round-trip ----
def verify(blob_id, expected_hex, retries=6, wait=5):
    """Aggregator may lag behind the publisher; retry a few times."""
    last = None
    for i in range(retries):
        try:
            req = urllib.request.Request(WAL_AGG + blob_id,
                headers={'User-Agent': 'deepedge-loop/1.0'})
            with urllib.request.urlopen(req, timeout=30) as r:
                data = r.read()
            return hashlib.sha256(data).hexdigest() == expected_hex
        except urllib.error.HTTPError as e:
            last = e
            if e.code in (403, 404, 425):  # not propagated yet
                print(f'    (aggregator not ready, retry {i+1}/{retries} after {wait}s)')
                time.sleep(wait)
                continue
            raise
    print(f'    verify gave up after {retries} tries: {last}')
    return False

# ---- 5. enforce + record on-chain ----
def enforce_and_record(amount, hash_hex, blob_id):
    # decision_hash and blob_id are both Strings now -> pass as quoted args
    cmd = [
        'sui', 'client', 'ptb',
        '--move-call', f'{PKG}::mandate::authorize_with_decision',
        f'@{MANDATE}', f'{amount}', f'"{hash_hex}"', f'"{blob_id}"',
        '--assign', 'receipt',
        '--move-call', f'{PKG}::mandate::record_decision_and_consume',
        f'@{MANDATE}', 'receipt',
        '--gas-budget', '50000000',
    ]
    return subprocess.run(cmd, capture_output=True, text=True)

def main():
    if not ANTHROPIC_KEY:
        sys.exit('source ~/.anthropic_key first')
    force_amount = int(sys.argv[1]) if len(sys.argv) > 1 else None
    oid_arg = sys.argv[2] if len(sys.argv) > 2 else None

    print('== 1. OBSERVE ==')
    oid, oracle, near, cal = observe(oid_arg)
    print(f"  {oracle['underlying_asset']} {oracle['expiry_iso']} strike {near['strike_usd']}")

    print('== 2. REASON (Claude) ==')
    verdict, inputs = reason(oracle, near, cal)
    print('  ' + json.dumps(verdict, ensure_ascii=False))

    print('== 3. BUILD DECISION RECORD ==')
    record = {
        'market': {'oracle_id': oid, 'asset': oracle['underlying_asset'],
                   'expiry': oracle['expiry_iso'], 'strike_usd': near['strike_usd']},
        'inputs': {'fair_up': inputs['fair_up'], 'fair_down': inputs['fair_down'],
                   'calibration_bucket': inputs['bucket']},
        'claude_model': MODEL,
        'verdict': verdict,
        'ts': int(time.time()),
    }
    record_bytes = json.dumps(record, sort_keys=True, separators=(',', ':')).encode()
    hash_hex = hashlib.sha256(record_bytes).hexdigest()
    print(f'  sha256: {hash_hex}')

    print('== 4. STORE ON WALRUS ==')
    blob_id = store_walrus(record_bytes)
    print(f'  blobId: {blob_id}')

    print('== 5. VERIFY ROUND-TRIP ==')
    ok = verify(blob_id, hash_hex)
    print(f'  walrus blob hashes back to on-chain hash: {ok}')

    # decide amount
    if force_amount is not None:
        amount = force_amount
    elif verdict['recommendation'] in ('BET_UP', 'BET_DOWN'):
        amount = int(500000 * float(verdict.get('confidence', 0.5)))  # <=0.5 DUSDC
    else:
        amount = 0

    print('== 6. ENFORCE + RECORD ON-CHAIN ==')
    if amount > 0:
        res = enforce_and_record(amount, hash_hex, blob_id)
        if res.returncode == 0:
            print(f'  recorded on-chain: amount={amount}, hash+blobId emitted')
            tail = [l for l in res.stdout.splitlines() if 'Digest' in l or 'Status' in l]
            for l in tail[:2]:
                print('   ', l.strip())
        else:
            print('  on-chain record FAILED:')
            print(res.stderr[-500:])
    else:
        print('  NO_BET -> reasoning stored on Walrus, no on-chain spend.')
        print('  (the decision is still verifiable; we just did not bet)')

    print('== CYCLE COMPLETE ==')

if __name__ == '__main__':
    main()
