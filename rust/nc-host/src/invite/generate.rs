//! Invite code generation.

use std::collections::HashSet;

use rand::Rng;

use super::wordlist::WORDLIST;

pub fn generate_invite_code(existing_codes: &HashSet<String>) -> String {
    let mut rng = rand::thread_rng();
    loop {
        let code = random_code(&mut rng);
        if !existing_codes.contains(&code) {
            return code;
        }
    }
}

fn random_code<R: Rng>(rng: &mut R) -> String {
    let i = rng.gen_range(0..WORDLIST.len());
    let j = rng.gen_range(0..WORDLIST.len());
    format!("{}-{}", WORDLIST[i], WORDLIST[j])
}
