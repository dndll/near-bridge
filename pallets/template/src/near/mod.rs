use self::{
	block_header::ApprovalInner,
	hash::CryptoHash,
	views::{
		LightClientBlockLiteView, LightClientBlockView, ValidatorStakeView,
		ValidatorStakeViewScaleHax,
	},
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
	pub next_bps: Option<(CryptoHash, Vec<ValidatorStakeViewScaleHax>)>,
}

impl LightClientState {
	// TODO: needs syncing

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

		// (1) The block was already verified
		if block_view.inner_lite.height <= self.head.inner_lite.height {
			println!("Block has already been verified");
			return false
		}

		// (2)
		if ![self.head.inner_lite.epoch_id, self.head.inner_lite.next_epoch_id]
			.contains(&block_view.inner_lite.epoch_id)
		{
			println!("Block is not in the current or next epoch");
			return false
		}

		// (3) Same as next epoch and no new set, covering N + 2
		if block_view.inner_lite.epoch_id == self.head.inner_lite.next_epoch_id &&
			block_view.next_bps.is_none()
		{
			println!("Block is in the next epoch but no new set");
			return false
		}

		// (4) and (5)
		let mut total_stake = 0;
		let mut approved_stake = 0;

		for (maybe_signature, block_producer) in
			block_view.approvals_after_next.iter().zip(epoch_block_producers.iter())
		{
			total_stake += block_producer.stake;

			if let Some(signature) = maybe_signature {
				println!(
					"Checking if signature {} and message {:?} was signed by {}",
					signature, approval_message, block_producer.public_key
				);
				approved_stake += block_producer.stake;
				if !signature.verify(&approval_message, &block_producer.public_key) {
					println!("Signature is invalid");
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
				println!("Next block producers hash is invalid");
				return false
			}

			self.next_bps = Some((
				block_view.inner_lite.next_epoch_id,
				next_bps.into_iter().map(|s| s.clone().into()).collect(),
			));
		}

		self.head = LightClientBlockLiteView::from(block_view.to_owned());

		true
	}
}

#[cfg(test)]
mod tests {
	use super::{
		client::{JsonRpcResult, NearRpcResult},
		*,
	};
	use ed25519_dalek::Verifier;
	use near_crypto::Signature;
	use serde_json;

	fn get_file(file: &str) -> JsonRpcResult {
		serde_json::from_reader(std::fs::File::open(file).unwrap()).unwrap()
	}

	fn get_previous_header() -> LightClientBlockView {
		if let NearRpcResult::NextBlock(header) = get_file("fixtures/2_previous_epoch.json").result
		{
			header
		} else {
			panic!("Expected block header")
		}
	}

	fn get_next_header() -> LightClientBlockView {
		if let NearRpcResult::NextBlock(header) = get_file("fixtures/1_current_epoch.json").result {
			header
		} else {
			panic!("Expected block header")
		}
	}

	#[test]
	fn test_sig_verification() {
		let previous = get_previous_header();
		let next = get_next_header();
	}

	fn get_previous() -> LightClientBlockView {
		let s: JsonRpcResult =
			serde_json::from_reader(std::fs::File::open("fixtures/2_previous_epoch.json").unwrap())
				.unwrap();
		if let NearRpcResult::NextBlock(header) = s.result {
			header
		} else {
			panic!("Expected block header")
		}
	}
	fn get_previous_previous() -> LightClientBlockView {
		let s: JsonRpcResult =
			serde_json::from_reader(std::fs::File::open("fixtures/3_previous_epoch.json").unwrap())
				.unwrap();
		if let NearRpcResult::NextBlock(header) = s.result {
			header
		} else {
			panic!("Expected block header")
		}
	}

	fn get_next_bps() -> Vec<ValidatorStakeView> {
		get_previous().next_bps.unwrap()
	}

	fn get_next() -> LightClientBlockView {
		let s: JsonRpcResult =
			serde_json::from_reader(std::fs::File::open("fixtures/1_current_epoch.json").unwrap())
				.unwrap();
		if let NearRpcResult::NextBlock(header) = s.result {
			header
		} else {
			panic!("Expected block header")
		}
	}

	#[test]
	fn test_deserialize_into_hax_correctly() {
		let bps = get_next_bps();

		let hax: Vec<ValidatorStakeViewScaleHax> = bps.iter().map(|s| s.clone().into()).collect();

		let bps_again: Vec<ValidatorStakeView> =
			hax.iter().map(|s| s.clone().into()).collect::<Vec<_>>();

		assert_eq!(bps, bps_again);
	}

	#[test]
	fn fake_validate_and_update_next() {
		let last_verified_head = get_previous();
		// FIXME: need to get these, currently verifying against the next bps is why this test fails
		let current_epoch_bps = get_previous_previous().next_bps.unwrap();

		let mut state =
			LightClientState { head: last_verified_head.clone().into(), next_bps: None };

		let new_head = get_next();

		let did_pass = state.validate_and_update_head(&new_head, current_epoch_bps);
		assert!(did_pass);
	}

	#[test]
	fn test_can_verify_one_sig() {
		let mut state = LightClientState { head: get_previous().clone().into(), next_bps: None };
		let (_, _, approval_message) =
			state.reconstruct_light_client_block_view_fields(&get_next().clone().into());

		let signature = get_previous().approvals_after_next[0].clone().unwrap();
		if let Signature::ED25519(signature) = signature {
			let bps = get_previous_previous().next_bps.unwrap();

			// FIXME: need to get these, currently verifying against the next bps is why this test
			// fails
			let signer = bps[0].public_key.unwrap_as_ed25519();
			let signer = ed25519_dalek::PublicKey::from_bytes(&signer.0).unwrap();

			signer.verify(&approval_message[..], &signature).unwrap();
		} else {
			panic!("Expected ed25519 signature")
		}
	}
}
