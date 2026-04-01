use nc_gate::serve::request_subscription_filter;
use nostr_sdk::Timestamp;

#[test]
fn request_subscription_filter_is_broad_and_since_bound() {
    let filter = request_subscription_filter(Timestamp::from(4321));
    let json = serde_json::to_value(&filter).unwrap();

    assert_eq!(json["kinds"][0], 30501);
    assert_eq!(json["kinds"][1], 30504);
    assert_eq!(json["kinds"][2], 30507);
    assert_eq!(json["kinds"][3], 30510);
    assert_eq!(json["since"], 4321);
}
