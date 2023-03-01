use borsh::{BorshDeserialize, BorshSerialize};

use super::{
	hash::CryptoHash,
	types::{BlockHeight, EpochId, MerkleHash},
};

#[derive(BorshSerialize, BorshDeserialize, serde::Serialize, Debug, Clone, Eq, PartialEq)]
pub struct BlockHeaderInnerLite {
	/// Height of this block.
	pub height: BlockHeight,
	/// Epoch start hash of this block's epoch.
	/// Used for retrieving validator information
	pub epoch_id: EpochId,
	pub next_epoch_id: EpochId,
	/// Root hash of the state at the previous block.
	pub prev_state_root: MerkleHash,
	/// Root of the outcomes of transactions and receipts.
	pub outcome_root: MerkleHash,
	/// Timestamp at which the block was built (number of non-leap-nanoseconds since January 1,
	/// 1970 0:00:00 UTC).
	pub timestamp: u64,
	/// Hash of the next epoch block producers set
	pub next_bp_hash: CryptoHash,
	/// Merkle root of block hashes up to the current block.
	pub block_merkle_root: CryptoHash,
}

/// The part of the block approval that is different for endorsements and skips
#[derive(BorshSerialize, BorshDeserialize, serde::Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ApprovalInner {
	Endorsement(CryptoHash),
	Skip(BlockHeight),
}
