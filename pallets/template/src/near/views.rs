use borsh::BorshSerialize;
use codec::{Decode, Encode};
use near_crypto::{PublicKey, Signature};

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

#[derive(PartialEq, Eq, Debug, Clone, serde::Serialize, serde::Deserialize, BorshSerialize)]
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

impl From<BlockHeaderInnerLiteView> for BlockHeaderInnerLite {
	fn from(view: BlockHeaderInnerLiteView) -> Self {
		BlockHeaderInnerLite {
			height: view.height,
			epoch_id: EpochId(view.epoch_id),
			next_epoch_id: EpochId(view.next_epoch_id),
			prev_state_root: view.prev_state_root,
			outcome_root: view.outcome_root,
			timestamp: view.timestamp_nanosec,
			next_bp_hash: view.next_bp_hash,
			block_merkle_root: view.block_merkle_root,
		}
	}
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct LightClientBlockLiteView {
	pub prev_block_hash: CryptoHash,
	pub inner_rest_hash: CryptoHash,
	pub inner_lite: BlockHeaderInnerLiteView,
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
