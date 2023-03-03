use super::dec_format;
use borsh::BorshSerialize;
/// Account identifier. Provides access to user's state.
use derive_more::{AsRef as DeriveAsRef, From as DeriveFrom};
use near_crypto::PublicKey;

pub type AccountId = String;
use super::hash::CryptoHash;
/// Hash used by a struct implementing the Merkle tree.
pub type MerkleHash = CryptoHash;
/// Validator identifier in current group.
pub type ValidatorId = u64;
/// Mask which validators participated in multi sign.
pub type ValidatorMask = Vec<bool>;
/// StorageUsage is used to count the amount of storage used by a contract.
pub type StorageUsage = u64;
/// StorageUsageChange is used to count the storage usage within a single contract call.
pub type StorageUsageChange = i64;
/// Nonce for transactions.
pub type Nonce = u64;
/// Height of the block.
pub type BlockHeight = u64;
/// Height of the epoch.
pub type EpochHeight = u64;
/// Shard index, from 0 to NUM_SHARDS - 1.
pub type ShardId = u64;
/// Balance is type for storing amounts of tokens.
pub type Balance = u128;
/// Gas is a type for storing amount of gas.
pub type Gas = u64;
/// Hash used by to store state root.
pub type StateRoot = CryptoHash;

/// Epoch identifier -- wrapped hash, to make it easier to distinguish.
/// EpochId of epoch T is the hash of last block in T-2
/// EpochId of first two epochs is 0
#[derive(
	Debug,
	Clone,
	Default,
	Hash,
	Eq,
	PartialEq,
	PartialOrd,
	Ord,
	DeriveAsRef,
	serde::Serialize,
	serde::Deserialize,
	BorshSerialize,
)]
#[as_ref(forward)]
pub struct EpochId(pub CryptoHash);

/// Stores validator and its stake for two consecutive epochs.
/// It is necessary because the blocks on the epoch boundary need to contain approvals from both
/// epochs.
#[derive(serde::Serialize, Debug, Clone, PartialEq, Eq)]
pub struct ApprovalStake {
	/// Account that stakes money.
	pub account_id: AccountId,
	/// Public key of the proposed validator.
	pub public_key: PublicKey,
	/// Stake / weight of the validator.
	pub stake_this_epoch: Balance,
	pub stake_next_epoch: Balance,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum BlockId {
	Height(BlockHeight),
	Hash(CryptoHash),
}
