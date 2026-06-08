# How to re-run the formal verification

Tools (already installed on this server):
- sui-prover 1.5.3  (~/.cargo/bin/sui-prover)
- Boogie 3.5.6      (~/.dotnet/tools/boogie)
- Z3 4.16.0         (~/.local/bin/z3)   <- must be 4.11+, NOT the apt 4.8.12
- .NET 8.0          (/usr/bin/dotnet)

Run:
    cd /root/deepedge/move-spec
    export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$HOME/.dotnet/tools:$PATH"
    export BOOGIE_EXE="$HOME/.dotnet/tools/boogie"
    export Z3_EXE="$HOME/.local/bin/z3"
    sui-prover -p .

Expected: `Verification successful`
(authorize_respects_cap_spec: for all u64 inputs, a successful authorize()
returns a receipt with amount <= per_bet_cap and == the requested amount.)

Why move-spec/ is separate from move/:
The prover pulls asymptotic-code/sui (rev next), which clashes with the
deepbook_predict -> deepbook address (0x0 vs 0xdee9). move-spec/ is a
predict-free copy of the mandate core, so the proof runs without that
conflict. The authorize logic is identical to production move/sources/mandate.move.
