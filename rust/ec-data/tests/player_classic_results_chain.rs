use ec_data::PlayerRecord;

#[test]
fn classic_results_chain_state_round_trips() {
    let mut player = PlayerRecord::new_zeroed();
    assert!(!player.has_classic_results_chain_state());

    player.set_classic_results_chain_state(true, 23);
    assert_eq!(player.classic_results_chain_flag_raw(), 1);
    assert_eq!(player.classic_results_chain_next_free_raw(), 23);
    assert_eq!(&player.raw[0x3A..0x3C], &[0, 0]);
    assert_eq!(&player.raw[0x3E..0x40], &[0, 0]);
    assert!(player.has_classic_results_chain_state());

    player.set_classic_results_chain_state(false, 0);
    assert_eq!(player.classic_results_chain_flag_raw(), 0);
    assert_eq!(player.classic_results_chain_next_free_raw(), 0);
    assert!(!player.has_classic_results_chain_state());
}
