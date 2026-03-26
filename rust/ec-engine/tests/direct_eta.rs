use ec_engine::estimate_direct_eta;

#[test]
fn direct_eta_matches_preserved_starbase_travel_time() {
    assert_eq!(estimate_direct_eta([15, 13], [2, 12], 1, true), 16);
}

#[test]
fn direct_eta_returns_zero_for_arrived_target() {
    assert_eq!(estimate_direct_eta([6, 5], [6, 5], 1, true), 0);
}
