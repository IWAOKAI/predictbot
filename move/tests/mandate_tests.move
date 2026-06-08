#[test_only]
module deepedge_mandate::mandate_tests {
    use deepedge_mandate::mandate::{
        Self,
        Mandate,
        EPerBetExceeded,
        EBudgetExceeded,
        EMandateInactive,
    };
    use sui::test_scenario as ts;
    use std::string;

    const OWNER: address = @0xA11CE;

    // Helper: create a shared Mandate and return to the scenario.
    fun new_mandate(scenario: &mut ts::Scenario, per_bet: u64, budget: u64) {
        mandate::create_mandate(per_bet, budget, ts::ctx(scenario));
    }

    // 1. A fresh mandate starts at spent=0, active, with the given limits.
    #[test]
    fun test_create_mandate_initial_state() {
        let mut scenario = ts::begin(OWNER);
        new_mandate(&mut scenario, 2_000_000, 10_000_000);
        ts::next_tx(&mut scenario, OWNER);
        let m = ts::take_shared<Mandate>(&scenario);
        assert!(mandate::spent(&m) == 0, 0);
        assert!(mandate::is_active(&m), 1);
        assert!(mandate::per_bet_cap(&m) == 2_000_000, 2);
        assert!(mandate::total_budget(&m) == 10_000_000, 3);
        ts::return_shared(m);
        ts::end(scenario);
    }

    // 2. authorize within the cap succeeds and record increments spent.
    #[test]
    fun test_authorize_and_record_increments_spent() {
        let mut scenario = ts::begin(OWNER);
        new_mandate(&mut scenario, 2_000_000, 10_000_000);
        ts::next_tx(&mut scenario, OWNER);
        let mut m = ts::take_shared<Mandate>(&scenario);
        let r = mandate::authorize(&m, 1_000_000);
        assert!(mandate::receipt_amount(&r) == 1_000_000, 0);
        mandate::record_and_consume(&mut m, r);
        assert!(mandate::spent(&m) == 1_000_000, 1);
        ts::return_shared(m);
        ts::end(scenario);
    }

    // 3. authorize above the per-bet cap aborts (EPerBetExceeded).
    #[test]
    #[expected_failure(abort_code = EPerBetExceeded)]
    fun test_authorize_exceeds_per_bet_cap() {
        let mut scenario = ts::begin(OWNER);
        new_mandate(&mut scenario, 2_000_000, 10_000_000);
        ts::next_tx(&mut scenario, OWNER);
        let m = ts::take_shared<Mandate>(&scenario);
        let r = mandate::authorize(&m, 3_000_000); // > per_bet_cap
        mandate::record_and_consume(&mut ts_drop(m), r);
        abort 42
    }

    // 4. cumulative spend above total_budget aborts (EBudgetExceeded).
    #[test]
    #[expected_failure(abort_code = EBudgetExceeded)]
    fun test_authorize_exceeds_budget() {
        let mut scenario = ts::begin(OWNER);
        new_mandate(&mut scenario, 9_000_000, 10_000_000);
        ts::next_tx(&mut scenario, OWNER);
        let mut m = ts::take_shared<Mandate>(&scenario);
        // first bet 8M (ok), second 8M would exceed budget 10M
        let r1 = mandate::authorize(&m, 8_000_000);
        mandate::record_and_consume(&mut m, r1);
        let r2 = mandate::authorize(&m, 8_000_000); // 8M + 8M > 10M budget
        mandate::record_and_consume(&mut m, r2);
        ts::return_shared(m);
        ts::end(scenario);
    }

    // 5. kill switch: an inactive mandate rejects authorize (EMandateInactive).
    #[test]
    #[expected_failure(abort_code = EMandateInactive)]
    fun test_inactive_mandate_rejects() {
        let mut scenario = ts::begin(OWNER);
        new_mandate(&mut scenario, 2_000_000, 10_000_000);
        ts::next_tx(&mut scenario, OWNER);
        let mut m = ts::take_shared<Mandate>(&scenario);
        mandate::set_active(&mut m, false, ts::ctx(&mut scenario)); // kill switch
        let r = mandate::authorize(&m, 1_000_000); // should abort
        mandate::record_and_consume(&mut m, r);
        ts::return_shared(m);
        ts::end(scenario);
    }

    // 6. decision-bound path: authorize_with_decision + record increments spent
    //    and carries the reasoning hash + Walrus blob id.
    #[test]
    fun test_decision_bound_record() {
        let mut scenario = ts::begin(OWNER);
        new_mandate(&mut scenario, 2_000_000, 10_000_000);
        ts::next_tx(&mut scenario, OWNER);
        let mut m = ts::take_shared<Mandate>(&scenario);
        let h = string::utf8(b"b0cc3776e2a47b2311cc28d891331339");
        let blob = string::utf8(b"xfV3G5J13l7WRxXgJ3tw7ywFMhntZxz8");
        let r = mandate::authorize_with_decision(&m, 500_000, h, blob);
        mandate::record_decision_and_consume(&mut m, r);
        assert!(mandate::spent(&m) == 500_000, 0);
        ts::return_shared(m);
        ts::end(scenario);
    }

    // 7. decision path also enforces the per-bet cap.
    #[test]
    #[expected_failure(abort_code = EPerBetExceeded)]
    fun test_decision_exceeds_cap() {
        let mut scenario = ts::begin(OWNER);
        new_mandate(&mut scenario, 2_000_000, 10_000_000);
        ts::next_tx(&mut scenario, OWNER);
        let m = ts::take_shared<Mandate>(&scenario);
        let h = string::utf8(b"deadbeef");
        let blob = string::utf8(b"blob");
        let r = mandate::authorize_with_decision(&m, 3_000_000, h, blob);
        mandate::record_decision_and_consume(&mut ts_drop(m), r);
        abort 42
    }

    // small helper so the abort-path tests type-check while still aborting
    fun ts_drop(m: Mandate): Mandate { m }
}
