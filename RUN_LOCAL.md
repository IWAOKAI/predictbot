# Running DeepEdge locally (for demo / development)

## Backend (Rust API, port 3000)
    cd /root/deepedge
    source ~/.anthropic_key          # needed so /api/agent/run can call Claude
    nohup ./target/release/deepedge-server > /tmp/backend.log 2>&1 &
    # health check: curl http://localhost:3000/health

## Frontend (Next.js, port 3001)
    cd /root/deepedge/frontend
    PORT=3001 nohup npm run start > /tmp/frontend.log 2>&1 &
    # the 6 screens: / /overview /insights /portfolio /market/[id] /agent

## SSH tunnel from the Mac
    ssh -o ServerAliveInterval=60 -N \
      -L 3001:localhost:3001 -L 3000:localhost:3000 \
      root@167.179.119.190
    # then open http://localhost:3001 in the browser

## The demo money shot
Open http://localhost:3001/agent and click "Run one cycle":
observe -> Strategist proposes -> Risk Officer reviews (usually VETO,
citing the 0.40-0.50 calibration bucket) -> store on Walrus -> verify
hash -> enforce on-chain (no spend if vetoed). The Mandate panel shows
the on-chain cap/budget/spent and the formally-verified note.

## Agent endpoints
- POST /api/agent/run    -> runs scripts/deepedge_loop_api.py --json
- GET  /api/agent/status -> on-chain Mandate state
