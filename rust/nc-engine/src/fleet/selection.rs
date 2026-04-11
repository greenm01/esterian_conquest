#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectedFleetRef {
    pub fleet_record_index_1_based: usize,
    pub fleet_number: u16,
    pub coords: [u8; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckedFleetMergePlan {
    pub host_record_index_1_based: usize,
    pub host_fleet_number: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckedFleetTransferPlan {
    pub donor_record_index_1_based: usize,
    pub donor_fleet_number: u16,
    pub host_record_index_1_based: usize,
    pub host_fleet_number: u16,
}

pub fn resolve_checked_fleet_merge_plan(
    selected: &[SelectedFleetRef],
) -> Result<CheckedFleetMergePlan, &'static str> {
    if selected.len() < 2 {
        return Err("Check at least two fleets to merge.");
    }
    let anchor = selected[0].coords;
    if selected.iter().any(|row| row.coords != anchor) {
        return Err("Checked fleets must all be in the same sector to merge.");
    }
    let host = selected
        .iter()
        .min_by_key(|row| row.fleet_number)
        .expect("checked merge selection should be non-empty");
    Ok(CheckedFleetMergePlan {
        host_record_index_1_based: host.fleet_record_index_1_based,
        host_fleet_number: host.fleet_number,
    })
}

pub fn resolve_checked_fleet_transfer_plan(
    selected: &[SelectedFleetRef],
    highlighted_record_index_1_based: Option<usize>,
) -> Result<CheckedFleetTransferPlan, &'static str> {
    if selected.len() != 2 {
        return Err("Check exactly two fleets to transfer ships.");
    }
    if selected[0].coords != selected[1].coords {
        return Err("Checked fleets must be in the same sector to transfer ships.");
    }
    let donor = highlighted_record_index_1_based
        .and_then(|record_index| {
            selected
                .iter()
                .find(|row| row.fleet_record_index_1_based == record_index)
                .copied()
        })
        .unwrap_or_else(|| {
            *selected
                .iter()
                .min_by_key(|row| row.fleet_number)
                .expect("checked transfer selection should be non-empty")
        });
    let host = selected
        .iter()
        .find(|row| row.fleet_record_index_1_based != donor.fleet_record_index_1_based)
        .copied()
        .expect("checked transfer should contain a host fleet");
    Ok(CheckedFleetTransferPlan {
        donor_record_index_1_based: donor.fleet_record_index_1_based,
        donor_fleet_number: donor.fleet_number,
        host_record_index_1_based: host.fleet_record_index_1_based,
        host_fleet_number: host.fleet_number,
    })
}
