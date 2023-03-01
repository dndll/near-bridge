use sha2::{Digest, Sha256};
use std::{collections::HashMap, convert::TryInto};

use self::{
	block_header::ApprovalInner,
	views::{LightClientBlockView, ValidatorStakeView},
};

pub mod block_header;
pub mod hash;
pub mod merkle;
pub mod proof;
pub mod types;
pub mod views;

pub struct LightClientState {
	pub head: LightClientBlockView,
}

impl LightClientState {
	fn reconstruct_light_client_block_view_fields(
		&mut self,
		block_view: &LightClientBlockView,
	) -> ([u8; 32], [u8; 32], Vec<u8>) {
		let current_block_hash = Sha256::new()
			.chain(
				Sha256::new()
					.chain(
						Sha256::new()
							.chain(&block_view.inner_lite.to_vec())
							.chain(&block_view.inner_rest_hash)
							.finalize(),
					)
					.chain(&block_view.prev_block_hash)
					.finalize(),
			)
			.finalize()
			.try_into()
			.unwrap();

		let next_block_hash = Sha256::new()
			.chain(&block_view.next_block_inner_hash)
			.chain(&current_block_hash)
			.finalize()
			.try_into()
			.unwrap();

		let mut approval_message =
			borsh::to_vec(&ApprovalInner::Endorsement(next_block_hash)).unwrap();
		approval_message.extend(&((block_view.inner_lite.height + 2) as u32).to_le_bytes());

		(current_block_hash, next_block_hash, approval_message)
	}

	// TODO: introduce own state
	fn validate_and_update_head(
		&mut self,
		block_view: &LightClientBlockView,
		epoch_block_producers_map: &mut HashMap<u64, Vec<ValidatorStakeView>>,
	) -> bool {
		let (current_block_hash, next_block_hash, approval_message) =
			self.reconstruct_light_client_block_view_fields(block_view);

		// (1)
		if block_view.inner_lite.height <= self.head.inner_lite.height {
			return false
		}

		// (2)
		if ![self.head.inner_lite.epoch_id, self.head.inner_lite.next_epoch_id]
			.contains(&block_view.inner_lite.epoch_id)
		{
			return false
		}

		// (3)
		if block_view.inner_lite.epoch_id == self.head.inner_lite.next_epoch_id &&
			block_view.next_bps.is_none()
		{
			return false
		}

		// (4) and (5)
		let mut total_stake = 0;
		let mut approved_stake = 0;

		let epoch_block_producers =
			epoch_block_producers_map.get(&block_view.inner_lite.epoch_id).unwrap();
		for (maybe_signature, block_producer) in
			block_view.approvals_after_next.iter().zip(epoch_block_producers.iter())
		{
			total_stake += block_producer.stake;

			if let Some(signature) = maybe_signature {
				approved_stake += block_producer.stake;
				if !verify_signature(&block_producer.public_key, &signature, &approval_message) {
					return false
				}
			}
		}

		let threshold = total_stake * 2 / 3;
		if approved_stake <= threshold {
			return false
		}

		// (6)
		if let Some(next_bps) = &block_view.next_bps {
			if Sha256::digest(&borsh::to_vec(&next_bps).unwrap()) !=
				block_view.inner_lite.next_bp_hash
			{
				return false
			}

			epoch_block_producers_map.insert(block_view.inner_lite.next_epoch_id, next_bps.clone());
		}

		self.head = block_view;

		true
	}
}
