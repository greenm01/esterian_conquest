pub fn print_usage() {
    println!("Usage:");
    println!("  ec-sysop new-game <target_dir> [--players <1-25>] [--seed <u64>]");
    println!("  ec-sysop maint <dir> [turns]");
}

pub fn print_new_game_usage() {
    println!("Usage:");
    println!("  ec-sysop new-game <target_dir> [--players <1-25>] [--seed <u64>]");
}

pub fn print_maint_usage() {
    println!("Usage:");
    println!("  ec-sysop maint <dir> [turns]");
}
