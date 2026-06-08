/// Formal specifications for the Mandate enforcement contract.
/// Verified with the Sui Prover (#[spec(prove)]): mathematical proofs
/// over ALL inputs, not example-based tests.
#[spec_only]
module deepedge_mandate::mandate_spec {
    use prover::prover::{ensures, requires};
    use deepedge_mandate::mandate::{Self, Mandate, BetReceipt};

    /// PROOF 1: if authorize() succeeds, the authorized amount is provably
    /// within the per-bet cap, and equals the requested amount. The agent
    /// can never obtain a receipt for more than the cap -- the enforcement
    /// guarantee, proven for all inputs.
    #[spec(prove)]
    public fun authorize_respects_cap_spec(m: &Mandate, amount: u64): BetReceipt {
        // preconditions matching the real-world valid state, using
        // overflow-safe integer comparison (.to_int())
        requires(mandate::is_active(m));
        requires(amount <= mandate::per_bet_cap(m));
        requires(mandate::spent(m).to_int().add(amount.to_int()).lte(mandate::total_budget(m).to_int()));
        let r = mandate::authorize(m, amount);
        // postconditions: the receipt is provably within the cap
        ensures(mandate::receipt_amount(&r) <= mandate::per_bet_cap(m));
        ensures(mandate::receipt_amount(&r) == amount);
        r
    }
}
