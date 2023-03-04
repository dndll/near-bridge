#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[cfg(test)]
mod mock;

mod near;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_runtime::offchain::{
		storage_lock::{BlockAndTime, StorageLock},
		Duration,
	};

	use crate::near::{client::NearRpcClient, views::LightClientBlockLiteView};

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::storage]
	#[pallet::getter(fn light_client_head)]
	pub type LightClientHead<T> = StorageValue<_, LightClientBlockLiteView>;

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
		fn offchain_worker(block_number: T::BlockNumber) {
			// log::info!("Hello from pallet-near.");

			// Here we are showcasing various techniques used when running off-chain workers (ocw)
			// 1. Sending signed transaction from ocw
			// 2. Sending unsigned transaction from ocw
			// 3. Sending unsigned transactions with signed payloads from ocw
			// 4. Fetching JSON via http requests in ocw

			// let modu = block_number.try_into().map_or(TX_TYPES, |bn: usize| (bn as u32) %
			// TX_TYPES); let result = match modu {
			// 	0 => Self::offchain_signed_tx(block_number),
			// 	1 => Self::offchain_unsigned_tx(block_number),
			// 	2 => Self::offchain_unsigned_tx_signed_payload(block_number),
			// 	3 => Self::fetch_remote_info(),
			// 	_ => Err(Error::<T>::UnknownOffchainMux),
			// };

			// if let Err(e) = result {
			// 	log::error!("offchain_worker error: {:?}", e);
			// }
			let latest_header = LightClientHead::<T>::get().unwrap();
			let head = NearRpcClient.fetch_latest_header(&format!("{}", latest_header.hash()));
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::call_index(0)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn do_something(origin: OriginFor<T>, something: u32) -> DispatchResult {
			// // Check that the extrinsic was signed and get the signer.
			// // This function will return an error if the extrinsic is not signed.
			// // https://docs.substrate.io/main-docs/build/origins/
			// let who = ensure_signed(origin)?;

			// // Update storage.
			// <Something<T>>::put(something);

			// // Emit an event.
			// Self::deposit_event(Event::SomethingStored { something, who });
			// // Return a successful DispatchResultWithPostInfo
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Check if we have fetched the data before. If yes, we can use the cached version
		///   stored in off-chain worker storage `storage`. If not, we fetch the remote info and
		///   write the info into the storage for future retrieval.
		fn fetch_remote_info() -> Result<(), Error<T>> {
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
			Ok(())
		}
	}
}
