use borsh::{BorshDeserialize, BorshSerialize};

use super::{
	block_header::BlockHeaderInnerLite,
	hash::{hash, CryptoHash},
	merkle::combine_hash,
	types::{Balance, BlockHeight, EpochId},
};

#[derive(
	PartialEq,
	Eq,
	Debug,
	Clone,
	BorshDeserialize,
	BorshSerialize,
	serde::Serialize,
	serde::Deserialize,
)]
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
	BorshDeserialize,
	BorshSerialize,
	serde::Serialize,
	serde::Deserialize,
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

impl From<BlockHeader> for BlockHeaderInnerLiteView {
	fn from(header: BlockHeader) -> Self {
		match header {
			BlockHeader::BlockHeaderV1(header) => BlockHeaderInnerLiteView {
				height: header.inner_lite.height,
				epoch_id: header.inner_lite.epoch_id.0,
				next_epoch_id: header.inner_lite.next_epoch_id.0,
				prev_state_root: header.inner_lite.prev_state_root,
				outcome_root: header.inner_lite.outcome_root,
				timestamp: header.inner_lite.timestamp,
				timestamp_nanosec: header.inner_lite.timestamp,
				next_bp_hash: header.inner_lite.next_bp_hash,
				block_merkle_root: header.inner_lite.block_merkle_root,
			},
			BlockHeader::BlockHeaderV2(header) => BlockHeaderInnerLiteView {
				height: header.inner_lite.height,
				epoch_id: header.inner_lite.epoch_id.0,
				next_epoch_id: header.inner_lite.next_epoch_id.0,
				prev_state_root: header.inner_lite.prev_state_root,
				outcome_root: header.inner_lite.outcome_root,
				timestamp: header.inner_lite.timestamp,
				timestamp_nanosec: header.inner_lite.timestamp,
				next_bp_hash: header.inner_lite.next_bp_hash,
				block_merkle_root: header.inner_lite.block_merkle_root,
			},
			BlockHeader::BlockHeaderV3(header) => BlockHeaderInnerLiteView {
				height: header.inner_lite.height,
				epoch_id: header.inner_lite.epoch_id.0,
				next_epoch_id: header.inner_lite.next_epoch_id.0,
				prev_state_root: header.inner_lite.prev_state_root,
				outcome_root: header.inner_lite.outcome_root,
				timestamp: header.inner_lite.timestamp,
				timestamp_nanosec: header.inner_lite.timestamp,
				next_bp_hash: header.inner_lite.next_bp_hash,
				block_merkle_root: header.inner_lite.block_merkle_root,
			},
		}
	}
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct LightClientBlockLiteView {
	pub prev_block_hash: CryptoHash,
	pub inner_rest_hash: CryptoHash,
	pub inner_lite: BlockHeaderInnerLiteView,
}

impl From<BlockHeader> for LightClientBlockLiteView {
	fn from(header: BlockHeader) -> Self {
		Self {
			prev_block_hash: *header.prev_hash(),
			inner_rest_hash: hash(&header.inner_rest_bytes()),
			inner_lite: header.into(),
		}
	}
}
impl LightClientBlockLiteView {
	pub fn hash(&self) -> CryptoHash {
		let block_header_inner_lite: BlockHeaderInnerLite = self.inner_lite.clone().into();
		combine_hash(
			&combine_hash(
				&hash(&block_header_inner_lite.try_to_vec().unwrap()),
				&self.inner_rest_hash,
			),
			&self.prev_block_hash,
		)
	}
}

#[derive(
	BorshSerialize,
	BorshDeserialize,
	serde::Serialize,
	serde::Deserialize,
	Debug,
	Clone,
	Eq,
	PartialEq,
)]
#[serde(tag = "validator_stake_struct_version")]
pub enum ValidatorStakeView {
	V1(ValidatorStakeViewV1),
}

#[derive(
	BorshSerialize,
	BorshDeserialize,
	Debug,
	Clone,
	Eq,
	PartialEq,
	serde::Serialize,
	serde::Deserialize,
)]
pub struct ValidatorStakeViewV1 {
	pub account_id: AccountId,
	pub public_key: PublicKey,
	#[serde(with = "dec_format")]
	pub stake: Balance,
}
