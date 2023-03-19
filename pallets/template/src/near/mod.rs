use crate::near::{block_header::BlockHeaderInnerLite, hash::sha256};

use self::{
	block_header::ApprovalInner,
	hash::CryptoHash,
	views::{
		BlockHeaderInnerLiteView, LightClientBlockLiteView, LightClientBlockView,
		ValidatorStakeView, ValidatorStakeViewScaleHax,
	},
};
use crate::near::hash::borsh as borshit;
use codec::{Decode, Encode};
use serialize::{base64_format, dec_format};
use sp_runtime::sp_std::{prelude::*, vec};

pub mod block_header;
pub mod client;
pub mod hash;
pub mod merkle;
pub mod proof;
pub mod serialize;
pub mod signature;
pub mod types;
pub mod views;
pub mod errors;

#[derive(Debug, Clone, Encode, Decode, scale_info::TypeInfo)]
pub struct LightClientState {
	pub head: LightClientBlockLiteView,
	pub next_bps: Option<(CryptoHash, Vec<ValidatorStakeViewScaleHax>)>,
}

macro_rules! cvec {
	($($x:expr),*) => {
		{
			let mut temp_vec = Vec::new();
			$(
				temp_vec.extend_from_slice(&$x);
			)*
			temp_vec
		}
	};
}
impl LightClientState {
	// TODO: needs syncing

	fn inner_lite_hash(ilv: &BlockHeaderInnerLiteView) -> CryptoHash {
		let full_inner: BlockHeaderInnerLite = ilv.clone().into();
		CryptoHash::hash_borsh(full_inner)
	}

	fn calculate_current_block_hash(block_view: &LightClientBlockView) -> CryptoHash {
		let inner_light_hash = Self::inner_lite_hash(&block_view.inner_lite);
		let inner_hash = sha256(&cvec!(inner_light_hash.0, block_view.inner_rest_hash.0));
		CryptoHash::hash_bytes(&cvec!(inner_hash, block_view.prev_block_hash.as_bytes().to_vec()))
	}

	fn next_block_hash(
		&self,
		current_block_hash: &CryptoHash,
		block_view: &LightClientBlockView,
	) -> CryptoHash {
		CryptoHash::hash_bytes(&cvec!(block_view.next_block_inner_hash.0, current_block_hash.0))
	}
	fn reconstruct_light_client_block_view_fields(
		&mut self,
		block_view: &LightClientBlockView,
	) -> ([u8; 32], [u8; 32], Vec<u8>) {
		let current_block_hash = Self::calculate_current_block_hash(block_view);

		let next_block_hash: CryptoHash = self.next_block_hash(&current_block_hash, block_view);

		let endorsement = ApprovalInner::Endorsement(next_block_hash);

		let approval_message =
			cvec!(borshit(&endorsement), (block_view.inner_lite.height + 2).to_le_bytes());

		log::info!("Current block hash: {}", current_block_hash);
		log::info!("Next block hash: {}", next_block_hash);
		log::info!("Approval message: {:?}", approval_message);
		(*current_block_hash.as_bytes(), next_block_hash.into(), approval_message)
	}

	pub fn validate_and_update_head(
		&mut self,
		block_view: &LightClientBlockView,
		epoch_block_producers: Vec<ValidatorStakeView>,
	) -> bool {
		let (_, _, approval_message) = self.reconstruct_light_client_block_view_fields(block_view);

		// (1) The block was already verified
		if block_view.inner_lite.height <= self.head.inner_lite.height {
			log::info!("Block has already been verified");
			return false
		}

		// (2)
		if ![self.head.inner_lite.epoch_id, self.head.inner_lite.next_epoch_id]
			.contains(&block_view.inner_lite.epoch_id)
		{
			log::info!("Block is not in the current or next epoch");
			return false
		}

		// (3) Same as next epoch and no new set, covering N + 2
		if block_view.inner_lite.epoch_id == self.head.inner_lite.next_epoch_id &&
			block_view.next_bps.is_none()
		{
			log::info!("Block is in the next epoch but no new set");
			return false
		}

		// (4) and (5)
		let mut total_stake = 0;
		let mut approved_stake = 0;

		for (maybe_signature, block_producer) in
			block_view.approvals_after_next.iter().zip(epoch_block_producers.iter())
		{
			total_stake += block_producer.stake();

			if let Some(signature) = maybe_signature {
				log::info!(
					"Checking if signature {} and message {:?} was signed by {}",
					signature,
					approval_message,
					block_producer.public_key()
				);
				approved_stake += block_producer.stake();
				if !signature.verify(&approval_message, &block_producer.public_key()) {
					log::info!("Signature is invalid");
					return false
				}
			}
		}
		log::info!("All signatures are valid");

		let threshold = total_stake * 2 / 3;
		if approved_stake <= threshold {
			return false
		}

		// (6)
		if let Some(next_bps) = &block_view.next_bps {
			let next_bps_hash = CryptoHash::hash_borsh(&next_bps);
			log::info!("Next block producers calculated hash: {}", next_bps_hash);
			log::info!("Next block producers hash: {}", block_view.inner_lite.next_bp_hash);

			if next_bps_hash != block_view.inner_lite.next_bp_hash {
				log::info!("Next block producers hash is invalid");
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
	use core::str::FromStr;

	use super::{
		block_header::BlockHeaderInnerLite,
		client::{JsonRpcResult, NearRpcResult},
		views::BlockHeaderInnerLiteView,
		*,
	};
	use borsh::{BorshDeserialize, BorshSerialize};
	use ed25519_dalek::Verifier;
	use near_crypto::{KeyType, Signature};
	use serde_json;
	use sp_core::bytes::from_hex;

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

	fn get_current() -> LightClientBlockView {
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

		let new_head = get_current();

		let did_pass = state.validate_and_update_head(&new_head, current_epoch_bps);
		assert!(did_pass);
	}

	fn get_epochs() -> Vec<(u32, LightClientBlockView, CryptoHash)> {
		vec![
			(
				1,
				get_previous_previous(),
				CryptoHash::from_str("B35Jn6mLXACRcsf6PATMixqgzqJZd71JaNh1LScJjFuJ").unwrap(),
			),
			(
				2,
				get_previous(),
				CryptoHash::from_str("Doy7Y7aVMgN8YhdAseGBMHNmYoqzWsXszqJ7MFLNMcQ7").unwrap(),
			), // Doy7Y7aVMgN8YhdAseGBMHNmYoqzWsXszqJ7MFLNMcQ7
			(
				3,
				get_current(),
				CryptoHash::from_str("3tyxRRBgbYTo5DYd1LpX3EZtEiRYbDAAji6kcsf9QRge").unwrap(),
			), // 3tyxRRBgbYTo5DYd1LpX3EZtEiRYbDAAji6kcsf9QRge
		]
	}

	#[test]
	fn test_headers() {
		let headers_by_epoch = get_epochs();
		let next_epoch_id = headers_by_epoch[1].1.inner_lite.epoch_id.clone();

		let mut state = LightClientState {
			head: headers_by_epoch[0].1.clone().into(),
			next_bps: Some((
				next_epoch_id,
				headers_by_epoch[0]
					.1
					.next_bps
					.clone()
					.unwrap()
					.into_iter()
					.map(Into::into)
					.collect(),
			)),
		};

		let (current, _, approval_message) =
			state.reconstruct_light_client_block_view_fields(&headers_by_epoch[1].1);
		assert_eq!(current, headers_by_epoch[1].2 .0);

		let signature = headers_by_epoch[1].1.approvals_after_next[0].clone().unwrap();
		if let Signature::ED25519(signature) = signature {
			let first_validator = &get_previous_previous().next_bps.unwrap()[0];
			log::info!("first_validator: {:?}", first_validator);

			// FIXME: need to get these, currently verifying against the next bps is why this test
			// fails
			let signer = first_validator.public_key().unwrap_as_ed25519();
			let signer = ed25519_dalek::PublicKey::from_bytes(&signer.0).unwrap();

			signer.verify(&approval_message[..], &signature).unwrap();
		} else {
			panic!("Expected ed25519 signature")
		}
	}

	#[test]
	fn test_validate() {
		let headers_by_epoch = get_epochs();
		let next_epoch_id = headers_by_epoch[1].1.inner_lite.epoch_id.clone();

		let mut state = LightClientState {
			head: headers_by_epoch[0].1.clone().into(),
			next_bps: Some((
				next_epoch_id,
				headers_by_epoch[0]
					.1
					.next_bps
					.clone()
					.unwrap()
					.into_iter()
					.map(Into::into)
					.collect(),
			)),
		};

		assert!(state.validate_and_update_head(
			&headers_by_epoch[1].1.clone(),
			headers_by_epoch[0].1.next_bps.clone().unwrap(),
		));
	}

	// #[test]
	// fn test_can_verify_one_sig() {
	// 	let prev_hash =
	// 		CryptoHash::from_str("7aLAqDbJtwYQTVJz8xHTcRUgTYDGcYGkBptPCNXBrpSA").unwrap();

	// 	let file = "fixtures/well_known_header.json";
	// 	let head: BlockHeaderInnerLite =
	// 		serde_json::from_reader(std::fs::File::open(file).unwrap()).unwrap();

	// 	let view: LightClientBlockLiteView = LightClientBlockView {
	// 		inner_lite: BlockHeaderInnerLiteView::from(head.clone()),
	// 		inner_rest_hash,
	// 		prev_block_hash: prev_hash,
	// 		// Not needed for this test
	// 		approvals_after_next: vec![],
	// 		next_bps: None,
	// 		next_block_inner_hash: CryptoHash::default(),
	// 	}
	// 	.into();

	// 	let mut state = LightClientState { head: view, next_bps: None };
	// 	let (current_block_hash, next_block_hash, approval_message) =
	// 		state.reconstruct_light_client_block_view_fields(&get_next().clone().into());

	// 	let signature = ed25519_dalek::Signature::from_bytes(&[0; 64]).unwrap();
	// 	if let Signature::ED25519(signature) = signature {
	// 		let first_validator = &get_previous_previous().next_bps.unwrap()[0];
	// 		log::info!("first_validator: {:?}", first_validator);

	// 		// FIXME: need to get these, currently verifying against the next bps is why this test
	// 		// fails
	// 		let signer = first_validator.public_key.unwrap_as_ed25519();
	// 		let signer = ed25519_dalek::PublicKey::from_bytes(&signer.0).unwrap();

	// 		signer.verify(&approval_message[..], &signature).unwrap();
	// 	} else {
	// 		panic!("Expected ed25519 signature")
	// 	}
	// }

	#[test]
	fn test_inner_lite_hash_issue() {
		let file = "fixtures/well_known_header.json";
		let prev_hash =
			CryptoHash::from_str("BUcVEkMq3DcZzDGgeh1sb7FFuD86XYcXpEt25Cf34LuP").unwrap();
		let well_known_header: BlockHeaderInnerLite =
			serde_json::from_reader(std::fs::File::open(file).unwrap()).unwrap();
		let well_known_header_view: BlockHeaderInnerLiteView =
			BlockHeaderInnerLiteView::from(well_known_header.clone());

		// Manual borsh it
		let view_bytes = well_known_header.try_to_vec().unwrap();
		let expected = from_hex("040000000000000000000000000000000000000000000000000000000000000000000000000000009331b2bf4028e466f9d172fcbd0892cc063f0e8a0d8d751205b145c1bf573016a022eda2c13024e2cb4a11d2787b7670508bc627f3ff6c6d65e73ef476cb81ad66687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f292500aa8070ffa2c9160f64f9ad70f5c0e8b197af0d1e010a2e202c02f73e86d30a68b1002765501e27e89ea8e5e3226b50d8193eb15f7d80830451f11e91e48a6ed7bda87fdee83803").unwrap();
		assert_eq!(view_bytes, expected);

		let inner_light_hash = CryptoHash::hash_bytes(&view_bytes);
		let expected =
			CryptoHash::from_str("6u6qjC19Z2aDWujqdKf52u1FHCQSvpQ1af7Y4fdWKwzU").unwrap();
		assert_eq!(inner_light_hash, expected);
		assert_eq!(inner_light_hash, LightClientState::inner_lite_hash(&well_known_header_view));
	}

	#[test]
	fn test_can_create_current_hash() {
		let file = "fixtures/well_known_header.json";
		let prev_hash =
			CryptoHash::from_str("BUcVEkMq3DcZzDGgeh1sb7FFuD86XYcXpEt25Cf34LuP").unwrap();
		let well_known_header: BlockHeaderInnerLite =
			serde_json::from_reader(std::fs::File::open(file).unwrap()).unwrap();
		let well_known_header_view: BlockHeaderInnerLiteView =
			BlockHeaderInnerLiteView::from(well_known_header.clone());

		let inner_light_hash = LightClientState::inner_lite_hash(&well_known_header_view);

		// Inner rest hashing
		for (inner_rest, expected) in vec![
			(
				"FaU2VzTNqxfouDtkQWcmrmU2UdvtSES3rQuccnZMtWAC",
				"3ckGjcedZiN3RnvfiuEN83BtudDTVa9Pub4yZ8R737qt",
			),
			(
				"BEqJyfEYNNrKmhHToZTvbeYqVjoqSe2w2uTMMT9KDaqb",
				"Hezx56VTH815G6JTzWqJ7iuWxdR9X4ZqGwteaDF8q2z",
			),
			(
				"7Kg3Uqeg7XSoQ7qXbed5JUR48YE49EgLiuqBTRuiDq6j",
				"Finjr87adnUqpFHVXbmAWiVAY12EA9G4DfUw27XYHox",
			),
		] {
			let inner_rest_hash = CryptoHash::from_str(inner_rest).unwrap();
			let expected = CryptoHash::from_str(expected).unwrap();

			let inner_hash = sha256(&cvec!(inner_light_hash.0, inner_rest_hash.0));
			let current_hash =
				CryptoHash::hash_bytes(&cvec!(inner_hash, prev_hash.as_bytes().to_vec()));
			assert_eq!(current_hash, expected);
		}
	}

	#[test]
	fn test_can_recreate_current_block_hash() {
		let file = "fixtures/well_known_header.json";
		let prev_hash =
			CryptoHash::from_str("BUcVEkMq3DcZzDGgeh1sb7FFuD86XYcXpEt25Cf34LuP").unwrap();
		let well_known_header: BlockHeaderInnerLite =
			serde_json::from_reader(std::fs::File::open(file).unwrap()).unwrap();
		let well_known_header_view: BlockHeaderInnerLiteView =
			BlockHeaderInnerLiteView::from(well_known_header.clone());

		// Inner rest hashing
		for (inner_rest, expected) in vec![
			(
				"FaU2VzTNqxfouDtkQWcmrmU2UdvtSES3rQuccnZMtWAC",
				"3ckGjcedZiN3RnvfiuEN83BtudDTVa9Pub4yZ8R737qt",
			),
			(
				"BEqJyfEYNNrKmhHToZTvbeYqVjoqSe2w2uTMMT9KDaqb",
				"Hezx56VTH815G6JTzWqJ7iuWxdR9X4ZqGwteaDF8q2z",
			),
			(
				"7Kg3Uqeg7XSoQ7qXbed5JUR48YE49EgLiuqBTRuiDq6j",
				"Finjr87adnUqpFHVXbmAWiVAY12EA9G4DfUw27XYHox",
			),
		] {
			let inner_rest_hash = CryptoHash::from_str(inner_rest).unwrap();
			let expected = CryptoHash::from_str(expected).unwrap();

			let view = LightClientBlockView {
				inner_lite: well_known_header_view.clone(),
				inner_rest_hash,
				prev_block_hash: prev_hash,
				// Not needed for this test
				approvals_after_next: vec![],
				next_bps: None,
				next_block_inner_hash: CryptoHash::default(),
			};

			let current_hash = LightClientState::calculate_current_block_hash(&view);
			assert_eq!(current_hash, expected);
		}
	}
}
