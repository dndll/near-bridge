#![cfg_attr(not(feature = "std"), no_std)]
#![feature(error_in_core)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[cfg(test)]
mod mock;

pub mod crypto;
mod near;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use crate::near::{
		client::NearRpcClient,
		hash::CryptoHash,
		types::BlockHeight,
		views::{LightClientBlockLiteView, ValidatorStakeView, ValidatorStakeViewScaleHax},
		LightClientState,
	};
	use borsh::maybestd::format;
	use frame_support::pallet_prelude::*;
	use frame_system::{
		offchain::{
			AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
			Signer,
		},
		pallet_prelude::*,
	};
	use sp_runtime::{
		sp_std::{prelude::*, vec},
		DispatchResult,
	};

	pub const MAX_BLOCK_PRODUCERS: u32 = 1024;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The identifier type for an offchain worker.
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;

		type Call: From<Call<Self>>;
	}

	#[pallet::storage]
	#[pallet::getter(fn light_client_head)]
	pub type LightClientHead<T> = StorageValue<_, LightClientBlockLiteView>;

	#[pallet::storage]
	#[pallet::getter(fn block_producers)]
	pub type BlockProducersByEpoch<T> = StorageMap<
		_,
		Identity,
		CryptoHash,
		BoundedVec<ValidatorStakeViewScaleHax, ConstU32<MAX_BLOCK_PRODUCERS>>,
	>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored { something: u32, who: T::AccountId },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		/// Validate unsigned call to this module.
		///
		/// By default unsigned transactions are disallowed, but implementing the validator
		/// here we make sure that some particular calls (the ones produced by offchain worker)
		/// are being whitelisted and marked as valid.
		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			// Firstly let's check that we call the right function.
			if let Call::submit { head, bps } = call {
				// let signature_valid =
				// 	SignedPayload::<T>::verify::<T::AuthorityId>(payload, signature.clone());
				// if !signature_valid {
				// 	return InvalidTransaction::BadProof.into()
				// }
				// Self::validate_transaction_parameters(&payload.block_number, &payload.price)
				Ok(ValidTransaction {
					priority: 1,
					requires: sp_runtime::sp_std::vec![],
					provides: sp_runtime::sp_std::vec![],
					longevity: 1,
					propagate: false,
				})
			} else {
				InvalidTransaction::Call.into()
			}
			// else if let Call::submit_price_unsigned { block_number, price: new_price } = call {
			// 	Self::validate_transaction_parameters(block_number, new_price)
			// }
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Offchain Worker entry point.
		///
		/// By implementing `fn offchain_worker` you declare a new offchain worker.
		/// This function will be called when the node is fully synced and a new best block is
		/// succesfuly imported.
		/// Note that it's not guaranteed for offchain workers to run on EVERY block, there might
		/// be cases where some blocks are skipped, or for some the worker runs twice (re-orgs),
		/// so the code should be able to handle that.
		/// You can use `Local Storage` API to coordinate runs of the worker.
		fn offchain_worker(_block_number: T::BlockNumber) {
			let (mut state, bps) = if let Some(head) = LightClientHead::<T>::get() {
				let state = LightClientState { head, next_bps: None };
				let bps: Vec<ValidatorStakeView> =
					BlockProducersByEpoch::<T>::get(state.head.inner_lite.epoch_id)
						.unwrap()
						.into_iter()
						.map(|s| ValidatorStakeView::from(s))
						.collect();
				(state, bps)
			} else {
				// Bootstrap here
				log::info!("Bootstrapping light client");
				let start_block_hash = "BoswxxbPApgouVZNH37jKo6PF9WgrcqqgYjEW8tdXXPU";

				// get current block and store in `Started`
				let starting_head = NearRpcClient.fetch_latest_header(start_block_hash);
				log::info!("Got starting head: {:?}", starting_head.inner_lite.height);

				let bps = starting_head.next_bps.clone().unwrap();
				let state = LightClientState { head: starting_head.into(), next_bps: None };
				(state, bps)
			};

			// Here we will have a mechanism to only try to sync if needs be, otherwise we will go
			// through verification. Receipts to be verified should be stored
			// reverse-chronologically in a Dequeue, since we will likely already have verified
			// the earliest headers.
			// TODO: implement locking on the queue, or at least locking on the syncing aspect
			// TODO: then another worker can start verifying TXs
			let verification_queue: Vec<BlockHeight> = Vec::new();

			// determine if should sync by checking if last tx in the queue is newer than our
			// head

			// blockbend note
			// U2FsdGVkX1+ZBNtoxCa2YwdHh5ibQZp9rxbVeSbQmiHLE1xRzahvPEazun/
			// dBGIl6p2vTiP83xDw4iWHMfcth29WDXNHBRx1UiOssU1aSPumMd4ZlILOgSfxl9gZlfTrzMLy9yN2gqk58OpADl+RwVyc2TQC+NDbOgDZxQgsqFimSCyj72p5EqcOdfuIOFks!
			// e569e0f4f2a1239663f8161979251972ec246dae

			// U2FsdGVkX182uv/
			// cRqnqcIi+4Ms9ez3CIzGMbYyHJb7xSL2Wwl0zrLt0t7ZqIFGEYXp3PkCZ5VT+mgxVyTQyrCM2Nt9aFaiDMk6OMlFMs1nlH754TwMGb4yHGW7T53nmIlXTJC3SuJEoRl9AQwDJ/
			// 8ImX5fTBiJ61/Njt6TK6ARiBnzyulL9G2ncWZR0idvv!070f63246269818372f679634f76275e1fc53b23
			let last_is_newer = verification_queue
				.last()
				.map(|h| h > &state.head.inner_lite.height)
				.unwrap_or(false);

			let should_sync = last_is_newer || verification_queue.is_empty();
			log::info!("Should sync: {:?}", should_sync);

			if should_sync {
				log::info!("Syncing from head: {:?}", state.head.inner_lite.height);

				// TODO: if so start verifying from queue
				let new_head = NearRpcClient.fetch_latest_header(&format!("{}", state.head.hash()));

				if state.validate_and_update_head(&new_head, bps) {
					if let Err(e) = Self::try_submit(Some(state.head), state.next_bps) {
						log::error!("Failed to submit {:?}", e);
					}
				}
			} else {
				// TODO: start verifying from front of queue
				// TODO: also verify some from middle to avoid ddos
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Check if we have fetched the data before. If yes, we can use the cached version
		///   stored in off-chain worker storage `storage`. If not, we fetch the remote info and
		///   write the info into the storage for future retrieval.
		// fn fetch_remote_info() -> Result<(), Error<T>> {
		// // Create a reference to Local Storage value.
		// // Since the local storage is common for all offchain workers, it's a good practice
		// // to prepend our entry with the pallet name.
		// let s_info = StorageValueRef::persistent(b"near::hn-info");

		// // Local storage is persisted and shared between runs of the offchain workers,
		// // offchain workers may run concurrently. We can use the `mutate` function to
		// // write a storage entry in an atomic fashion.
		// //
		// // With a similar API as `StorageValue` with the variables `get`, `set`, `mutate`.
		// // We will likely want to use `mutate` to access
		// // the storage comprehensively.
		// //
		// if let Ok(Some(info)) = s_info.get::<HackerNewsInfo>() {
		// 	// hn-info has already been fetched. Return early.
		// 	log::info!("cached hn-info: {:?}", info);
		// 	return Ok(())
		// }

		// // Since off-chain storage can be accessed by off-chain workers from multiple runs,
		// it // is important to lock   it before doing heavy computations or write operations.
		// //
		// // There are four ways of defining a lock:
		// //   1) `new` - lock with default time and block exipration
		// //   2) `with_deadline` - lock with default block but custom time expiration
		// //   3) `with_block_deadline` - lock with default time but custom block expiration
		// //   4) `with_block_and_time_deadline` - lock with custom time and block expiration
		// // Here we choose the most custom one for demonstration purpose.
		// let mut lock = StorageLock::<BlockAndTime<Self>>::with_block_and_time_deadline(
		// 	b"near::lock",
		// 	LOCK_BLOCK_EXPIRATION,
		// 	Duration::from_millis(LOCK_TIMEOUT_EXPIRATION),
		// );

		// // We try to acquire the lock here. If failed, we know the `fetch_n_parse` part
		// inside // is being   executed by previous run of ocw, so the function just returns.
		// if let Ok(_guard) = lock.try_lock() {
		// 	match Self::fetch_n_parse() {
		// 		Ok(info) => {
		// 			s_info.set(&info);
		// 		},
		// 		Err(err) => return Err(err),
		// 	}
		// }
		// Ok(())
		// }
		#[pallet::weight(0)]
		#[pallet::call_index(0)]
		pub fn submit_header(
			origin: OriginFor<T>,
			head: LightClientBlockLiteView,
		) -> DispatchResult {
			let _who = ensure_root(origin)?;

			log::info!("Storing new head: {:?}", head);
			LightClientHead::<T>::put(head);

			Ok(())
		}

		#[pallet::weight(0)]
		#[pallet::call_index(1)]
		pub fn submit_bps(
			origin: OriginFor<T>,
			epoch: CryptoHash,
			next_bps: Vec<ValidatorStakeViewScaleHax>,
		) -> DispatchResult {
			let _who = ensure_root(origin)?;

			log::info!("Storing bps: {:?}", next_bps.len());
			let next_bps: BoundedVec<ValidatorStakeViewScaleHax, ConstU32<MAX_BLOCK_PRODUCERS>> =
				BoundedVec::try_from(next_bps).unwrap();
			BlockProducersByEpoch::<T>::insert(epoch, next_bps);

			Ok(())
		}
		#[pallet::weight(1_000_000)]
		#[pallet::call_index(2)]
		pub fn submit(
			origin: OriginFor<T>,
			head: Option<LightClientBlockLiteView>,
			bps: Option<(CryptoHash, Vec<ValidatorStakeViewScaleHax>)>,
		) -> DispatchResult {
			if let Some(head) = head {
				log::info!("Received request to submit head {}", head.inner_lite.height);
				LightClientHead::<T>::put(head);

				// Self::submit_header(origin.clone(), head)?
			}
			if let Some((epoch, next_bps)) = bps {
				log::info!(
					"Received request to submit bps of len {} for epoch {:?}",
					next_bps.len(),
					epoch
				);
				let next_bps: BoundedVec<
					ValidatorStakeViewScaleHax,
					ConstU32<MAX_BLOCK_PRODUCERS>,
				> = BoundedVec::try_from(next_bps).unwrap();
				BlockProducersByEpoch::<T>::insert(epoch, next_bps);
				// Self::submit_bps(origin, epoch, next_bps)?
			}
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn try_submit(
			head: Option<LightClientBlockLiteView>,
			bps: Option<(CryptoHash, Vec<ValidatorStakeViewScaleHax>)>,
		) -> DispatchResult {
			// We retrieve a signer and check if it is valid.
			//   Since this pallet only has one key in the keystore. We use `any_account()1 to
			//   retrieve it. If there are multiple keys and we want to pinpoint it,
			// `with_filter()` can be chained,   ref: https://substrate.dev/rustdocs/v3.0.0/frame_system/offchain/struct.Signer.html
			let signer = Signer::<T, T::AuthorityId>::any_account();
			frame_support::ensure!(
				signer.can_sign(),
				"No local accounts available. Consider adding one via author_insertKey RPC."
			);

			signer
				// .send_signed_transaction(|_acct| Call::submit {
				// 	head: head.clone(),
				// 	bps: bps.clone(),
				// })
				.submit_unsigned_transaction(Call::submit { head: head.clone(), bps: bps.clone() })
				.ok_or("Failed to send request")
				// .map(|x| x.1.unwrap())
				.map(|x| x.unwrap())
				.map_err(|e| e.into())
		}
	}
}
