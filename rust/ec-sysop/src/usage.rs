pub fn print_usage() {
    println!("Usage:");
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] new-game <target_dir> [--name <title>] [--players <1-25>] [--year <u16>] [--seed <u64>]"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] maint <dir> [turns]"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] maint-all [--config <path>]"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] settings <show|set|reserve|unreserve|import-kdl> ..."
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] host <games|status> ..."
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
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] nostr claim --dir <game_dir> --player <N> --npub <NPUB-OR-HEX>"
    );
}

pub fn print_new_game_usage() {
    println!("Usage:");
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] new-game <target_dir> [--name <title>] [--players <1-25>] [--year <u16>] [--seed <u64>]"
    );
}

pub fn print_maint_usage() {
    println!("Usage:");
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] maint <dir> [turns]"
    );
}

pub fn print_maint_all_usage() {
    println!("Usage:");
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] maint-all [--config <path>]"
    );
}

pub fn print_settings_usage() {
    println!("Usage:");
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] settings show --dir <game_dir>"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] settings set --dir <game_dir> [--game-name <title>] [--theme-key <key>] [--snoop on|off] [--session-max-idle <minutes>] [--session-minimum-time <minutes>] [--session-local-timeout on|off] [--session-remote-timeout on|off] [--inactivity-purge-after <turns>] [--inactivity-autopilot-after <turns>] [--maintenance-enabled on|off] [--maintenance-interval-minutes <minutes>] [--maintenance-next-due <unix-seconds>]"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] settings reserve --dir <game_dir> --player <N> --alias <alias>"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] settings unreserve --dir <game_dir> --player <N>"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] settings import-kdl --dir <game_dir> [--file <config.kdl>]"
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
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] nostr claim --dir <game_dir> --player <N> --npub <NPUB-OR-HEX>"
    );
}

pub fn print_host_usage() {
    println!("Usage:");
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] host games list [--config <path>]"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] host games add --dir <game_dir> [--config <path>]"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] host games remove --dir <game_dir> [--config <path>]"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] host status [--config <path>]"
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

pub fn print_nostr_claim_usage() {
    println!("Usage:");
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] nostr claim --dir <game_dir> --player <N> --npub <NPUB-OR-HEX>"
    );
}
