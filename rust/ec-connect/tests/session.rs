use ec_connect::cache::GameCache;
use ec_connect::connect::session::resolve_gate_npub;

#[test]
fn missing_gate_lookup_uses_picker_first_message() {
    let err = resolve_gate_npub("play.example.com", &GameCache::empty(), None)
        .expect_err("missing gate should return an error");

    assert!(err.contains("joined game list"));
    assert!(err.contains("invite code"));
    assert!(err.contains("picker"));
    assert!(!err.contains("--gate"));
}
