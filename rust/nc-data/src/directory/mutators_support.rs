use super::*;

pub(super) fn next_available_owned_fleet_local_slot(
    records: &[FleetRecord],
    owner_empire: u8,
) -> u16 {
    let mut owned_slots = records
        .iter()
        .filter(|fleet| fleet.owner_empire_raw() == owner_empire)
        .map(FleetRecord::local_slot_word_raw)
        .filter(|slot| *slot != 0)
        .collect::<Vec<_>>();
    owned_slots.sort_unstable();
    owned_slots.dedup();

    let mut next = 1u16;
    for slot in owned_slots {
        if slot == next {
            next = next.saturating_add(1);
        } else if slot > next {
            break;
        }
    }
    next
}

pub(super) fn next_available_global_fleet_id(records: &[FleetRecord]) -> u16 {
    let mut fleet_ids = records
        .iter()
        .map(FleetRecord::fleet_id_word_raw)
        .filter(|fleet_id| *fleet_id != 0)
        .collect::<Vec<_>>();
    fleet_ids.sort_unstable();
    fleet_ids.dedup();

    let mut next = 1u16;
    for fleet_id in fleet_ids {
        if fleet_id == next {
            next = next.saturating_add(1);
        } else if fleet_id > next {
            break;
        }
    }
    next
}

pub(super) fn total_starships(record: &FleetRecord) -> u32 {
    u32::from(record.battleship_count())
        + u32::from(record.cruiser_count())
        + u32::from(record.destroyer_count())
        + u32::from(record.troop_transport_count())
        + u32::from(record.scout_count())
        + u32::from(record.etac_count())
}

pub(super) fn fleet_has_combat_ships(record: &FleetRecord) -> bool {
    record.destroyer_count() > 0 || record.cruiser_count() > 0 || record.battleship_count() > 0
}

pub(super) fn rebuild_owner_fleet_chain(
    records: &mut [FleetRecord],
    player: &mut PlayerRecord,
    owner_empire: u8,
) {
    let mut owned = records
        .iter()
        .enumerate()
        .filter(|(_, fleet)| fleet.owner_empire_raw() == owner_empire)
        .map(|(idx, fleet)| (idx, fleet.local_slot_word_raw(), fleet.fleet_id_word_raw()))
        .collect::<Vec<_>>();
    owned.sort_unstable_by_key(|(_, local_slot, _)| *local_slot);

    player.set_fleet_chain_head_raw(owned.first().map(|(_, _, fleet_id)| *fleet_id).unwrap_or(0));

    for (position, (idx, _, _)) in owned.iter().enumerate() {
        let previous_id = position
            .checked_sub(1)
            .and_then(|prev| owned.get(prev))
            .map(|(_, _, fleet_id)| *fleet_id as u8)
            .unwrap_or(0);
        let next_id = owned
            .get(position + 1)
            .map(|(_, _, fleet_id)| *fleet_id)
            .unwrap_or(0);
        records[*idx].set_previous_fleet_id(previous_id);
        records[*idx].set_next_fleet_link_word_raw(next_id);
    }
}

pub(super) fn build_guard_starbase_base_record(
    coords: [u8; 2],
    base_id: u8,
    summary_word: u16,
    chain_word: u16,
    owner_empire: u8,
    tuple_a: [u8; 5],
    tuple_b: [u8; 5],
    tuple_c: [u8; 5],
) -> BaseRecord {
    let mut record = BaseRecord::new_zeroed();
    record.set_local_slot_raw(base_id);
    record.set_summary_word_raw(summary_word);
    record.set_base_id_raw(base_id);
    record.set_link_word_raw(0x0000);
    record.set_chain_word_raw(chain_word);
    record.set_coords_raw(coords);
    record.set_tuple_a_payload_raw(tuple_a);
    record.set_tuple_b_payload_raw(tuple_b);
    record.set_tuple_c_payload_raw(tuple_c);
    record.set_trailing_coords_raw(coords);
    record.set_owner_empire_raw(owner_empire);
    record
}
