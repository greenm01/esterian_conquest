pub fn print_usage() {
    println!("Usage:");
    println!("  ec-cli inspect [dir]");
    println!("  ec-cli inspect-messages [dir]");
    println!("  ec-cli map-export [dir] <player_record> <output_txt_path>");
    println!("  ec-cli sysop <subcommand> ...");
    println!(
        "  ec-cli sysop new-game <target_dir> [--players <1-25>] [--config <setup.kdl>] [--seed <u64>]"
    );
    println!(
        "  ec-cli sysop generate-gamestate <target_dir> <player_count> <year> [<homeworld_x>:<homeworld_y>...]"
    );
    println!("  ec-cli sysop init-canonical-four-player-start <target_dir>  # compatibility alias");
    println!("  ec-cli sysop maintenance-days [dir]");
    println!("  ec-cli sysop maintenance-days <dir> set <sun|mon|tue|wed|thu|fri|sat>...");
    println!("  ec-cli sysop snoop [dir]");
    println!("  ec-cli sysop snoop <dir> <on|off>");
    println!("  ec-cli sysop purge-after [dir]");
    println!("  ec-cli sysop purge-after <dir> <turns>");
    println!("  ec-cli sysop setup-programs [dir]");
    println!("  ec-cli sysop port-setup [dir]");
    println!("  ec-cli sysop flow-control <dir> <com1|com2|com3|com4> [on|off]");
    println!("  ec-cli sysop com-irq <dir> <com1|com2|com3|com4> [irq]");
    println!("  ec-cli sysop local-timeout <dir> [on|off]");
    println!("  ec-cli sysop remote-timeout <dir> [on|off]");
    println!("  ec-cli sysop max-key-gap <dir> [minutes]");
    println!("  ec-cli sysop minimum-time <dir> [minutes]");
    println!("  ec-cli sysop autopilot-after <dir> [turns]");
    println!("  ec-cli core-report [dir]");
    println!("  ec-cli core-diff-current-known-baseline [dir]");
    println!("  ec-cli core-diff-current-known-baseline-offsets [dir]");
    println!("  ec-cli core-diff-canonical-current-known-baseline [dir]");
    println!("  ec-cli core-diff-canonical-current-known-baseline-offsets [dir]");
    println!("  ec-cli core-report-canonical-transition-clusters [dir]");
    println!("  ec-cli core-report-canonical-transition-details [dir]");
    println!("  ec-cli core-validate [dir]");
    println!("  ec-cli core-validate-current-known-baseline [dir]");
    println!("  ec-cli core-sync-counts [dir]");
    println!("  ec-cli core-sync-baseline [dir]");
    println!("  ec-cli core-sync-current-known-baseline [dir]");
    println!("  ec-cli core-sync-canonical-current-known-baseline [dir]");
    println!("  ec-cli core-sync-initialized-fleets [dir]");
    println!("  ec-cli core-sync-initialized-planets [dir]");
    println!("  ec-cli core-init-current-known-baseline [source_dir] <target_dir>");
    println!("  ec-cli core-init-canonical-current-known-baseline [source_dir] <target_dir>");
    println!("  ec-cli headers [dir]");
    println!("  ec-cli maintenance-days [dir]");
    println!("  ec-cli maintenance-days <dir> set <sun|mon|tue|wed|thu|fri|sat>...");
    println!("  ec-cli snoop [dir]");
    println!("  ec-cli snoop <dir> <on|off>");
    println!("  ec-cli purge-after [dir]");
    println!("  ec-cli purge-after <dir> <turns>");
    println!(
        "  ec-cli fleet-order <dir> <fleet_record> <speed> <order_code> <target_x> <target_y> [aux0] [aux1]"
    );
    println!("  ec-cli fleet-order-report [dir] [fleet_record]");
    println!(
        "  ec-cli fleet-order-init <target_dir> <fleet_record> <speed> <order_code> <target_x> <target_y> [aux0] [aux1]"
    );
    println!(
        "  ec-cli fleet-order-batch-init <target_root> <fleet_record:speed:order:target_x:target_y[:aux0[:aux1]]>..."
    );
    println!("  ec-cli planet-build <dir> <planet_record> <build_slot_raw> <build_kind_raw>");
    println!("  ec-cli planet-owner <dir> <planet_record> <owner_slot>");
    println!("  ec-cli planet-name <dir> <planet_record> <name>");
    println!("  ec-cli planet-stats <dir> <planet_record> <armies> <batteries>");
    println!("  ec-cli planet-potential <dir> <planet_record> <p1_raw> <p2_raw>");
    println!("  ec-cli planet-stored <dir> <planet_record> <points>");
    println!("  ec-cli planet-init-original <dir>");
    println!("  ec-cli planet-build-report [dir] [planet_record]");
    println!(
        "  ec-cli planet-build-init <target_dir> <planet_record> <build_slot_raw> <build_kind_raw>"
    );
    println!(
        "  ec-cli planet-build-batch-init <target_root> <planet_record:build_slot_raw:build_kind_raw>..."
    );
    println!("  ec-cli guard-starbase-onebase <dir> <target_x> <target_y>");
    println!("  ec-cli guard-starbase-report <dir>");
    println!("  ec-cli guard-starbase-init [source_dir] <target_dir> <target_x> <target_y>");
    println!("  ec-cli guard-starbase-batch-init [source_dir] <target_root> <x:y> <x:y>...");
    println!("  ec-cli bombard-onefleet <dir> <target_x> <target_y> [ca] [dd]");
    println!("  ec-cli bombard-init [source_dir] <target_dir> <target_x> <target_y> <ca> <dd>");
    println!("  ec-cli bombard-batch-init [source_dir] <target_root> <x:y:ca:dd> ...");
    println!(
        "  ec-cli invade-onefleet <dir> <target_x> <target_y> [sc] [bb] [ca] [dd] [tt] [armies]"
    );
    println!(
        "  ec-cli invade-init [source_dir] <target_dir> <target_x> <target_y> <sc> <bb> <ca> <dd> <tt> <armies>"
    );
    println!(
        "  ec-cli invade-batch-init [source_dir] <target_root> <x:y:sc:bb:ca:dd:tt:armies> ..."
    );
    println!(
        "  ec-cli fleet-battle <dir> <battle_x> <battle_y> [f0_roe] [f0_bb] [f0_ca] [f0_dd] [f2_ca] [f2_dd] [f4_sc] [f4_bb] [f4_ca] [f8_loc_x] [f8_loc_y] [f8_sc] [f8_bb] [f8_ca] [p14_x] [p14_y] [p14_armies] [p14_batteries]"
    );
    println!(
        "  ec-cli fleet-battle-init [source_dir] <target_dir> <battle_x> <battle_y> <f0_roe> <f0_bb> <f0_ca> <f0_dd> <f2_ca> <f2_dd> <f4_sc> <f4_bb> <f4_ca> <f8_loc_x> <f8_loc_y> <f8_sc> <f8_bb> <f8_ca> <p14_x> <p14_y> <p14_armies> <p14_batteries>"
    );
    println!(
        "  ec-cli fleet-battle-batch-init [source_dir] <target_root> <bx:by:f0r:f0bb:f0ca:f0dd:f2ca:f2dd:f4sc:f4bb:f4ca:f8lx:f8ly:f8sc:f8bb:f8ca:p14x:p14y:p14a:p14b> ..."
    );
    println!(
        "  ec-cli econ <dir> <target_x> <target_y> [bb] [ca] [dd] [p14_x] [p14_y] [p14_armies] [p14_batteries]"
    );
    println!(
        "  ec-cli econ-init [source_dir] <target_dir> <target_x> <target_y> <bb> <ca> <dd> <p14_x> <p14_y> <p14_armies> <p14_batteries>"
    );
    println!(
        "  ec-cli econ-batch-init [source_dir] <target_root> <x:y:bb:ca:dd:p14x:p14y:p14a:p14b> ..."
    );
    println!("  ec-cli economy-report [dir] [player_record]");
    println!("  ec-cli economy-tax-probe-init [dir] [player_record] [tax_rate]");
    println!("  ec-cli ipbm-report <dir>");
    println!("  ec-cli player-tax <dir> <player_record> <rate>");
    println!("  ec-cli ipbm-zero <dir> <count>");
    println!("  ec-cli ipbm-record-set <dir> <record_index> <primary> <owner> <gate> <follow_on>");
    println!("  ec-cli ipbm-validate <dir>");
    println!("  ec-cli ipbm-init [source_dir] <target_dir> <count>");
    println!("  ec-cli ipbm-batch-init [source_dir] <target_root> <count> <count>...");
    println!("  ec-cli compliance-report <dir>");
    println!("  ec-cli compliance-batch-report <root>");
    println!(
        "  ec-cli scenario <dir> <fleet-order|planet-build|guard-starbase|ipbm|move|bombard|fleet-battle|invade|econ>"
    );
    println!(
        "  ec-cli scenario <dir> show <fleet-order|planet-build|guard-starbase|ipbm|move|bombard|fleet-battle|invade|econ>"
    );
    println!(
        "  ec-cli scenario <dir> compose <fleet-order|planet-build|guard-starbase|ipbm|move|bombard|fleet-battle|invade|econ>..."
    );
    println!("  ec-cli scenario <dir> list");
    println!("  ec-cli scenario-init-all [source_dir] <target_root>");
    println!(
        "  ec-cli scenario-init [source_dir] <target_dir> <fleet-order|planet-build|guard-starbase|ipbm|move|bombard|fleet-battle|invade|econ>"
    );
    println!(
        "  ec-cli scenario-init-replayable [source_dir] <target_dir> <fleet-order|planet-build|guard-starbase|ipbm|move|bombard|fleet-battle|invade|econ>"
    );
    println!(
        "  ec-cli scenario-init-compose [source_dir] <target_dir> <fleet-order|planet-build|guard-starbase|ipbm|move|bombard|fleet-battle|invade|econ>..."
    );
    println!(
        "  ec-cli validate <dir> <all|fleet-order|planet-build|guard-starbase|ipbm|move|bombard|fleet-battle|invade|econ>"
    );
    println!(
        "  ec-cli validate-preserved <dir> <all|fleet-order|planet-build|guard-starbase|ipbm|move|bombard|fleet-battle|invade|econ>"
    );
    println!(
        "  ec-cli compare-preserved <dir> <all|fleet-order|planet-build|guard-starbase|ipbm|move|bombard|fleet-battle|invade|econ>"
    );
    println!("  ec-cli match [dir]");
    println!("  ec-cli compare <left_dir> <right_dir>");
    println!("  ec-cli init [source_dir] <target_dir>");
    println!(
        "  ec-cli generate-gamestate <target_dir> <player_count> <year> [<homeworld_x>:<homeworld_y>...]"
    );
    println!("  ec-cli maint-rust <dir> [turns]");
    println!("  ec-cli maint-compare <dir> [turns]");
}
