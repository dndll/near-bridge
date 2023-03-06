use self::{
	block_header::ApprovalInner,
	hash::CryptoHash,
	views::{LightClientBlockLiteView, LightClientBlockView, ValidatorStakeView},
};
use codec::{Decode, Encode};
use serialize::{base64_format, dec_format};
use sha2::{digest::Update, Digest, Sha256};
use std::{collections::HashMap, convert::TryInto};

pub mod block_header;
pub mod client;
pub mod hash;
pub mod merkle;
pub mod proof;
pub mod serialize;
pub mod types;
pub mod views;

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo)]
pub struct LightClientState {
	pub head: LightClientBlockLiteView,
	pub next_block_producers: Option<(CryptoHash, Vec<ValidatorStakeView>)>,
}

impl LightClientState {
	fn reconstruct_light_client_block_view_fields(
		&mut self,
		block_view: &LightClientBlockView,
	) -> ([u8; 32], [u8; 32], Vec<u8>) {
		let current_block_hash = CryptoHash::hash_bytes(
			&Sha256::new()
				.chain(
					Sha256::new()
						.chain(&borsh::to_vec(&block_view.inner_lite).unwrap())
						.chain(&block_view.inner_rest_hash)
						.finalize(),
				)
				.chain(&block_view.prev_block_hash)
				.finalize(),
		);

		let next_block_hash: CryptoHash = CryptoHash::hash_bytes(
			&vec![
				block_view.next_block_inner_hash.as_bytes().to_vec(),
				current_block_hash.as_bytes().to_vec(),
			]
			.concat(),
		);

		let endorsement = ApprovalInner::Endorsement(next_block_hash);

		let mut approval_message = CryptoHash::hash_borsh(&endorsement).as_bytes().to_vec();
		approval_message.extend(&((block_view.inner_lite.height + 2) as u32).to_le_bytes());

		(*current_block_hash.as_bytes(), next_block_hash.into(), approval_message)
	}

	pub fn validate_and_update_head(
		&mut self,
		block_view: &LightClientBlockView,
		epoch_block_producers: Vec<ValidatorStakeView>,
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

		// .get(&block_view.inner_lite.epoch_id).unwrap();
		let epoch_block_producers = epoch_block_producers;
		for (maybe_signature, block_producer) in
			block_view.approvals_after_next.iter().zip(epoch_block_producers.iter())
		{
			total_stake += block_producer.stake;

			if let Some(signature) = maybe_signature {
				approved_stake += block_producer.stake;
				if !signature.verify(&approval_message, &block_producer.public_key) {
					return false
				}
			}
		}

		let threshold = total_stake * 2 / 3;
		if approved_stake <= threshold {
			return false
		}

		// (6)
		// FIXME: BUG HERE< NEEDS BORSCH SERIALIZE
		if let Some(next_bps) = &block_view.next_bps {
			if CryptoHash::hash_borsh(&next_bps) != block_view.inner_lite.next_bp_hash {
				return false
			}

			self.next_block_producers =
				Some((block_view.inner_lite.next_epoch_id, next_bps.clone()));
		}

		self.head = LightClientBlockLiteView::from(block_view.to_owned());

		true
	}
}
