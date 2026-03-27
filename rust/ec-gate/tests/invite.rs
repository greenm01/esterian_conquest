//! Tests for invite code generation and validation.

use std::collections::HashSet;

use rand::rngs::StdRng;
use rand::SeedableRng;

use ec_gate::invite::wordlist::WORDLIST;
use ec_gate::invite::{generate::generate_invite_code_with_rng, is_valid_invite_code};

#[test]
fn wordlist_has_1626_words() {
    assert_eq!(WORDLIST.len(), 1626);
}

#[test]
fn wordlist_all_lowercase() {
    for word in &WORDLIST {
        assert_eq!(*word, word.to_lowercase(), "word not lowercase: {word}");
    }
}

#[test]
fn wordlist_no_empty_words() {
    for word in &WORDLIST {
        assert!(!word.is_empty(), "empty word found in wordlist");
    }
}

#[test]
fn generated_code_has_two_wordlist_words() {
    let mut rng = StdRng::seed_from_u64(42);
    let existing = HashSet::new();
    let code = generate_invite_code_with_rng(&mut rng, &existing);
    assert!(
        is_valid_invite_code(&code),
        "generated code {code:?} failed is_valid_invite_code"
    );
}

#[test]
fn generated_code_format_is_word_hyphen_word() {
    let mut rng = StdRng::seed_from_u64(99);
    let existing = HashSet::new();
    let code = generate_invite_code_with_rng(&mut rng, &existing);
    let parts: Vec<&str> = code.splitn(2, '-').collect();
    assert_eq!(
        parts.len(),
        2,
        "code should have exactly one hyphen: {code:?}"
    );
    assert!(!parts[0].is_empty());
    assert!(!parts[1].is_empty());
}

#[test]
fn generator_avoids_existing_codes() {
    let mut rng = StdRng::seed_from_u64(7);
    // Pre-fill existing with the first code the seeded RNG would produce.
    let first = generate_invite_code_with_rng(&mut StdRng::seed_from_u64(7), &HashSet::new());
    let mut existing = HashSet::new();
    existing.insert(first.clone());
    let second = generate_invite_code_with_rng(&mut rng, &existing);
    assert_ne!(second, first, "generator must not return an existing code");
}

#[test]
fn is_valid_invite_code_accepts_known_words() {
    // Both words are in the wordlist (confirmed from wordlist.rs).
    assert!(is_valid_invite_code("abbey-zoom"));
    assert!(is_valid_invite_code("velvet-azure"));
}

#[test]
fn is_valid_invite_code_strips_relay_suffix() {
    assert!(is_valid_invite_code("abbey-zoom@play.example.com"));
}

#[test]
fn is_valid_invite_code_rejects_unknown_words() {
    assert!(!is_valid_invite_code("notaword-abbey"));
    assert!(!is_valid_invite_code("abbey-notaword"));
}

#[test]
fn is_valid_invite_code_rejects_single_word() {
    assert!(!is_valid_invite_code("abbey"));
}

#[test]
fn is_valid_invite_code_is_case_insensitive() {
    assert!(is_valid_invite_code("Abbey-Zoom"));
    assert!(is_valid_invite_code("ABBEY-ZOOM"));
}

#[test]
fn generated_codes_are_distinct_across_many() {
    // Generate 200 codes, all should be unique.
    let mut rng = StdRng::seed_from_u64(12345);
    let mut seen = HashSet::new();
    for _ in 0..200 {
        let code = generate_invite_code_with_rng(&mut rng, &seen);
        assert!(
            seen.insert(code.clone()),
            "duplicate code generated: {code:?}"
        );
    }
}
