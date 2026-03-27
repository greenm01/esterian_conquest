pub fn print_usage() {
    println!("Usage:");
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] new-game <target_dir> [--players <1-25>] [--seed <u64>]"
    );
    println!(
        "  ec-sysop [--log-file <path>] [--log-level <error|warn|info|debug|trace>] maint <dir> [turns]"
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
