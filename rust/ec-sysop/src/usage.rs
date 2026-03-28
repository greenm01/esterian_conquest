pub fn print_usage() {
    println!("Usage:");
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] new-game <target_dir> [--players <1-25>] [--seed <u64>]"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] maint <dir> [turns]"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] nostr init [--identity <path>]"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] nostr serve [--config <path>] [--identity <path>]"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] nostr migrate-roster --dir <game_dir>"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] nostr seats --dir <game_dir>"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] nostr reissue --dir <game_dir> --player <N>"
    );
}

pub fn print_new_game_usage() {
    println!("Usage:");
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] new-game <target_dir> [--players <1-25>] [--seed <u64>]"
    );
}

pub fn print_maint_usage() {
    println!("Usage:");
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] maint <dir> [turns]"
    );
}

pub fn print_nostr_usage() {
    println!("Usage:");
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] nostr init [--identity <path>]"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] nostr serve [--config <path>] [--identity <path>]"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] nostr migrate-roster --dir <game_dir>"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] nostr seats --dir <game_dir>"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] nostr reissue --dir <game_dir> --player <N>"
    );
}

pub fn print_nostr_init_usage() {
    println!("Usage:");
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] nostr init [--identity <path>]"
    );
}

pub fn print_nostr_serve_usage() {
    println!("Usage:");
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] nostr serve [--config <path>] [--identity <path>]"
    );
}

pub fn print_nostr_migrate_roster_usage() {
    println!("Usage:");
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] nostr migrate-roster --dir <game_dir>"
    );
}

pub fn print_nostr_seats_usage() {
    println!("Usage:");
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] nostr seats --dir <game_dir>"
    );
}

pub fn print_nostr_reissue_usage() {
    println!("Usage:");
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] nostr reissue --dir <game_dir> --player <N>"
    );
}
