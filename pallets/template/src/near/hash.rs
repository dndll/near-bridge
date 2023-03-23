use borsh::{maybestd::io::Write, BorshSerialize};
use core::{
	fmt,
	hash::{Hash, Hasher},
};
use serde::{Deserializer, Serializer};
use sha2::Digest;
use sp_runtime::sp_std::{prelude::*, vec};

#[derive(
	Copy,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	derive_more::AsRef,
	derive_more::AsMut,
	borsh::BorshDeserialize,
	borsh::BorshSerialize,
	codec::Encode,
	codec::Decode,
	scale_info::TypeInfo,
)]
#[as_ref(forward)]
#[as_mut(forward)]
pub struct CryptoHash(pub [u8; 32]);

pub fn sha256(bytes: &[u8]) -> Vec<u8> {
	CryptoHash(sha2::Sha256::digest(bytes).into()).0.to_vec()
}

pub fn borsh<T: BorshSerialize>(value: T) -> Vec<u8> {
	value.try_to_vec().unwrap()
}

impl CryptoHash {
	pub const fn new() -> Self {
		Self([0; 32])
	}

	/// Calculates hash of given bytes.
	pub fn hash_bytes(bytes: &[u8]) -> CryptoHash {
		CryptoHash(sha2::Sha256::digest(bytes).into())
	}

	/// Calculates hash of borsh-serialised representation of an object.
	///
	/// Note that using this function with an array may lead to unexpected
	/// results.  For example, `CryptoHash::hash_borsh(&[1u32, 2, 3])` hashes
	/// a representation of a `[u32; 3]` array rather than a slice.  It may be
	/// cleaner to use [`Self::hash_borsh_iter`] instead.
	pub fn hash_borsh<T: BorshSerialize>(value: T) -> CryptoHash {
		let mut bytes = Vec::new();
		value.serialize(&mut bytes).unwrap();
		CryptoHash(sha2::Sha256::digest(bytes).into())
	}

	pub const fn as_bytes(&self) -> &[u8; 32] {
		&self.0
	}

	/// Converts hash into base58-encoded string and passes it to given visitor.
	///
	/// The conversion is performed without any memory allocation.  The visitor
	/// is given a reference to a string stored on stack.  Returns whatever the
	/// visitor returns.
	fn to_base58_impl<Out>(self, visitor: impl FnOnce(&str) -> Out) -> Out {
		// base58-encoded string is at most 1.4 times longer than the binary
		// sequence.  We’re serialising 32 bytes so ⌈32 * 1.4⌉ = 45 should be
		// enough.
		let mut buffer = [0u8; 45];
		let len = bs58::encode(self).into(&mut buffer[..]).unwrap();
		let value = core::str::from_utf8(&buffer[..len]).unwrap();
		visitor(value)
	}

	/// Decodes base58-encoded string into a 32-byte hash.
	///
	/// Returns one of three results: success with the decoded CryptoHash,
	/// invalid length error indicating that the encoded value was too short or
	/// too long or other decoding error (e.g. invalid character).
	fn from_base58_impl(encoded: &str) -> Decode58Result {
		let mut result = Self::new();
		match bs58::decode(encoded).into(&mut result.0) {
			Ok(len) if len == result.0.len() => Decode58Result::Ok(result),
			Ok(_) | Err(bs58::decode::Error::BufferTooSmall) => Decode58Result::BadLength,
			Err(err) => Decode58Result::Err(err),
		}
	}
}

/// Result of decoding base58-encoded crypto hash.
enum Decode58Result {
	/// Decoding succeeded.
	Ok(CryptoHash),
	/// The decoded data has incorrect length; either too short or too long.
	BadLength,
	/// There have been other decoding errors; e.g. an invalid character in the
	/// input buffer.
	Err(bs58::decode::Error),
}

impl Default for CryptoHash {
	fn default() -> Self {
		Self::new()
	}
}

impl serde::Serialize for CryptoHash {
	fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
	where
		S: Serializer,
	{
		self.to_base58_impl(|encoded| serializer.serialize_str(encoded))
	}
}

/// Serde visitor for [`CryptoHash`].
///
/// The visitor expects a string which is then base58-decoded into a crypto
/// hash.
struct Visitor;

impl<'de> serde::de::Visitor<'de> for Visitor {
	type Value = CryptoHash;

	fn expecting(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		fmt.write_str("base58-encoded 256-bit hash")
	}

	fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Self::Value, E> {
		match CryptoHash::from_base58_impl(s) {
			Decode58Result::Ok(result) => Ok(result),
			Decode58Result::BadLength => Err(E::invalid_length(s.len(), &self)),
			Decode58Result::Err(err) => Err(E::custom(err)),
		}
	}
}

impl<'de> serde::Deserialize<'de> for CryptoHash {
	fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_str(Visitor)
	}
}

impl core::str::FromStr for CryptoHash {
	type Err = Box<dyn core::error::Error + Send + Sync>;

	/// Decodes base58-encoded string into a 32-byte crypto hash.
	fn from_str(encoded: &str) -> Result<Self, Self::Err> {
		match Self::from_base58_impl(encoded) {
			Decode58Result::Ok(result) => Ok(result),
			Decode58Result::BadLength => Err("incorrect length for hash".into()),
			Decode58Result::Err(err) => Err("BS58 decoding error".into()),
		}
	}
}

impl TryFrom<&[u8]> for CryptoHash {
	type Error = Box<dyn core::error::Error + Send + Sync>;

	fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
		Ok(CryptoHash(bytes.try_into()?))
	}
}

impl From<CryptoHash> for Vec<u8> {
	fn from(hash: CryptoHash) -> Vec<u8> {
		hash.0.to_vec()
	}
}

impl From<&CryptoHash> for Vec<u8> {
	fn from(hash: &CryptoHash) -> Vec<u8> {
		hash.0.to_vec()
	}
}

impl From<CryptoHash> for [u8; 32] {
	fn from(hash: CryptoHash) -> [u8; 32] {
		hash.0
	}
}

impl fmt::Debug for CryptoHash {
	fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(self, fmtr)
	}
}

impl fmt::Display for CryptoHash {
	fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.to_base58_impl(|encoded| fmtr.write_str(encoded))
	}
}

// This implementation is compatible with derived PartialEq.
// Custom PartialEq implementation was explicitly removed in #4220.
#[allow(clippy::derive_hash_xor_eq)]
impl Hash for CryptoHash {
	fn hash<H: Hasher>(&self, state: &mut H) {
		state.write(self.as_ref());
	}
}

/// Calculates a hash of a bytes slice.
///
/// # Examples
///
/// The example below calculates the hash of the indicated data.
///
/// ```
/// let data = [1, 2, 3];
/// let hash = near_primitives_core::hash::hash(&data);
/// ```
pub fn hash(data: &[u8]) -> CryptoHash {
	CryptoHash::hash_bytes(data)
}

#[cfg(test)]
mod tests {
	use super::*;
	use core::str::FromStr;

	#[derive(serde::Deserialize, serde::Serialize)]
	struct Struct {
		hash: CryptoHash,
	}
	
	#[test]
	fn test_base58_successes() {
		for (encoded, hash) in [
			("11111111111111111111111111111111", CryptoHash::new()),
			("CjNSmWXTWhC3EhRVtqLhRmWMTkRbU96wUACqxMtV1uGf", hash(&[0, 1, 2])),
		] {
			assert_eq!(encoded, hash.to_string());
			assert_eq!(hash, CryptoHash::from_str(encoded).unwrap());

			let json = format!("\"{}\"", encoded);
			assert_eq!(json, serde_json::to_string(&hash).unwrap());
			assert_eq!(hash, serde_json::from_str::<CryptoHash>(&json).unwrap());
		}
	}

	#[test]
	fn test_from_str_failures() {
		fn test(input: &str, want_err: &str) {
			match CryptoHash::from_str(input) {
				Ok(got) => panic!("‘{input}’ should have failed; got ‘{got}’"),
				Err(err) => {
					assert!(err.to_string().starts_with(want_err), "input: ‘{input}’; err: {err}")
				},
			}
		}

		// Invalid characters
		test("foo-bar-baz", "provided string contained invalid character '-' at byte 3");

		// Wrong length
		for encoded in &[
			"CjNSmWXTWhC3ELhRmWMTkRbU96wUACqxMtV1uGf".to_string(),
			"".to_string(),
			"1".repeat(31),
			"1".repeat(33),
			"1".repeat(1000),
		] {
			test(encoded, "incorrect length for hash");
		}
	}

	#[test]
	fn test_serde_deserialise_failures() {
		fn test(input: &str, want_err: &str) {
			match serde_json::from_str::<CryptoHash>(input) {
				Ok(got) => panic!("‘{input}’ should have failed; got ‘{got}’"),
				Err(err) => {
					assert!(err.to_string().starts_with(want_err), "input: ‘{input}’; err: {err}")
				},
			}
		}

		test("\"foo-bar-baz\"", "provided string contained invalid character");
		// Wrong length
		for encoded in &[
			"\"CjNSmWXTWhC3ELhRmWMTkRbU96wUACqxMtV1uGf\"".to_string(),
			"\"\"".to_string(),
			format!("\"{}\"", "1".repeat(31)),
			format!("\"{}\"", "1".repeat(33)),
			format!("\"{}\"", "1".repeat(1000)),
		] {
			test(encoded, "invalid length");
		}
	}
}
