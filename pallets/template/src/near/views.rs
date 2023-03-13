use borsh::{BorshDeserialize, BorshSerialize};
use codec::{Decode, Encode};
use near_crypto::{ED25519PublicKey, PublicKey, Secp256K1PublicKey, Signature};
use sp_runtime::Either;

use super::{
	block_header::BlockHeaderInnerLite,
	hash::{hash, CryptoHash},
	merkle::combine_hash,
	serialize::dec_format,
	types::{AccountId, Balance, BlockHeight, EpochId},
};

#[derive(PartialEq, Eq, Debug, Clone, serde::Serialize, serde::Deserialize, BorshSerialize)]
pub struct LightClientBlockView {
	pub prev_block_hash: CryptoHash,
	pub next_block_inner_hash: CryptoHash,
	pub inner_lite: BlockHeaderInnerLiteView,
	pub inner_rest_hash: CryptoHash,
	pub next_bps: Option<Vec<ValidatorStakeView>>,
	pub approvals_after_next: Vec<Option<Signature>>,
}

#[derive(
	PartialEq,
	Eq,
	Debug,
	Clone,
	serde::Serialize,
	serde::Deserialize,
	BorshSerialize,
	BorshDeserialize,
	codec::Encode,
	codec::Decode,
	scale_info::TypeInfo,
)]
pub struct BlockHeaderInnerLiteView {
	pub height: BlockHeight,
	pub epoch_id: CryptoHash,
	pub next_epoch_id: CryptoHash,
	pub prev_state_root: CryptoHash,
	pub outcome_root: CryptoHash,
	/// Legacy json number. Should not be used.
	pub timestamp: u64,
	#[serde(with = "dec_format")]
	pub timestamp_nanosec: u64,
	pub next_bp_hash: CryptoHash,
	pub block_merkle_root: CryptoHash,
}

impl From<BlockHeaderInnerLite> for BlockHeaderInnerLiteView {
	fn from(block_header: BlockHeaderInnerLite) -> Self {
		Self {
			height: block_header.height,
			epoch_id: block_header.epoch_id.0,
			next_epoch_id: block_header.next_epoch_id.0,
			prev_state_root: block_header.prev_state_root,
			outcome_root: block_header.outcome_root,
			timestamp: block_header.timestamp,
			timestamp_nanosec: block_header.timestamp,
			next_bp_hash: block_header.next_bp_hash,
			block_merkle_root: block_header.block_merkle_root,
		}
	}
}

#[derive(
	serde::Serialize,
	serde::Deserialize,
	Debug,
	Clone,
	codec::Encode,
	codec::Decode,
	scale_info::TypeInfo,
)]
pub struct LightClientBlockLiteView {
	pub prev_block_hash: CryptoHash,
	pub inner_rest_hash: CryptoHash,
	pub inner_lite: BlockHeaderInnerLiteView,
}

impl From<LightClientBlockView> for LightClientBlockLiteView {
	fn from(block: LightClientBlockView) -> Self {
		Self {
			prev_block_hash: block.prev_block_hash,
			inner_rest_hash: block.inner_rest_hash,
			inner_lite: block.inner_lite,
		}
	}
}

impl LightClientBlockLiteView {
	pub fn hash(&self) -> CryptoHash {
		let block_header_inner_lite: BlockHeaderInnerLite = self.inner_lite.clone().into();
		combine_hash(
			&combine_hash(
				&hash(&borsh::to_vec(&block_header_inner_lite).unwrap()),
				&self.inner_rest_hash,
			),
			&self.prev_block_hash,
		)
	}
}

#[derive(Debug, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize, BorshSerialize)]
pub struct ValidatorStakeView {
	pub account_id: AccountId,
	pub public_key: PublicKey,
	#[serde(with = "dec_format")]
	pub stake: Balance,
}

impl From<ValidatorStakeViewScaleHax> for ValidatorStakeView {
	fn from(value: ValidatorStakeViewScaleHax) -> Self {
		ValidatorStakeView {
			account_id: value.account_id,
			public_key: match value.public_key.len() {
				32 =>
					PublicKey::ED25519(ED25519PublicKey::try_from(&value.public_key[..]).unwrap()),
				64 => PublicKey::SECP256K1(
					Secp256K1PublicKey::try_from(&value.public_key[..]).unwrap(),
				),
				_ => panic!("Invalid public key length"),
			},
			stake: value.stake,
		}
	}
}

#[derive(
	Debug, Clone, Eq, PartialEq, codec::Encode, codec::Decode, scale_info::TypeInfo, BorshSerialize,
)]
pub struct ValidatorStakeViewScaleHax {
	pub account_id: AccountId,
	pub public_key: Vec<u8>,
	pub stake: Balance,
}

impl From<ValidatorStakeView> for ValidatorStakeViewScaleHax {
	fn from(view: ValidatorStakeView) -> Self {
		Self {
			account_id: view.account_id,
			public_key: view.public_key.key_data().to_vec(),
			stake: view.stake,
		}
	}
}
