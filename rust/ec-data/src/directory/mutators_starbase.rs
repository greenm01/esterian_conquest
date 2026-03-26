use super::*;

impl CoreGameData {
    pub fn set_starbase_destination(
        &mut self,
        player_index_1_based: usize,
        base_record_index_1_based: usize,
        destination: [u8; 2],
    ) -> Result<(), GameStateMutationError> {
        let owner_empire = player_index_1_based as u8;
        let base = self
            .bases
            .records
            .get_mut(base_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingBaseRecord {
                index_1_based: base_record_index_1_based,
            })?;
        if base.active_flag_raw() == 0 || base.owner_empire_raw() != owner_empire {
            return Err(GameStateMutationError::BaseOwnershipMismatch {
                player_index_1_based,
                base_record_index_1_based,
            });
        }
        base.set_trailing_coords_raw(destination);
        Ok(())
    }

    pub fn halt_starbase(
        &mut self,
        player_index_1_based: usize,
        base_record_index_1_based: usize,
    ) -> Result<(), GameStateMutationError> {
        let destination = self
            .bases
            .records
            .get(base_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingBaseRecord {
                index_1_based: base_record_index_1_based,
            })?
            .coords_raw();
        self.set_starbase_destination(player_index_1_based, base_record_index_1_based, destination)
    }
}
