use frame_support::sp_runtime::offchain::http;
use serde::{Deserialize, Serialize};
use sp_runtime::offchain::{
	http::{Method, Request},
	Duration,
};

use crate::{near::views::LightClientBlockView, Error};

pub const NEAR_RPC_ENDPOINT: &str = "https://rpc.mainnet.near.org";
const FETCH_TIMEOUT_PERIOD: u64 = 30000; // in milli-seconds
const LOCK_TIMEOUT_EXPIRATION: u64 = FETCH_TIMEOUT_PERIOD + 1000; // in milli-seconds
const LOCK_BLOCK_EXPIRATION: u32 = 3; // in block number

#[derive(Deserialize, Serialize, Default)]
pub struct JsonRpc {
	jsonrpc: String,
	method: String,
	params: NearRpcRequestParams,
	id: String,
}

impl JsonRpc {
	pub fn new(method: String, params: NearRpcRequestParams) -> Self {
		Self { jsonrpc: "2.0".to_string(), method, params, id: "pallet-near".to_string() }
	}
}

impl From<NearRpcRequestParams> for JsonRpc {
	fn from(params: NearRpcRequestParams) -> Self {
		let method = params.get_method_name();
		Self::new(method, params)
	}
}

#[derive(Deserialize, Serialize)]
pub enum LightClientProofParams {
	Transaction { transaction_hash: String, sender_id: String },
	Receipt { receipt_id: String, receiver_id: String },
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum NearRpcRequestParams {
	NextBlock {
		last_block_hash: String,
	},
	ExperimentalLightClientProof {
		#[serde(rename = "type")]
		kind: String,
		#[serde(flatten)]
		params: LightClientProofParams,
		light_client_head: String,
	},
}
impl Default for NearRpcRequestParams {
	fn default() -> Self {
		Self::NextBlock { last_block_hash: "".to_string() }
	}
}

impl NearRpcRequestParams {
	fn get_method_name(&self) -> String {
		match self {
			NearRpcRequestParams::NextBlock { .. } => "next_light_client_block".to_string(),
			NearRpcRequestParams::ExperimentalLightClientProof { .. } =>
				"EXPERIMENTAL_light_client_proof".to_string(),
		}
	}
}

pub struct NearRpcClient;

impl NearRpcClient {
	pub fn fetch_latest_header(&self, latest_verified: &str) {
		let request_body: JsonRpc =
			NearRpcRequestParams::NextBlock { last_block_hash: latest_verified.to_string() }.into();

		let request = Request::default()
			.method(Method::Post)
			.url(NEAR_RPC_ENDPOINT)
			.body(vec![serde_json::to_vec(&request_body).unwrap()])
			.add_header("Content-Type", "application/json");

		// Keeping the offchain worker execution time reasonable, so limiting the call to be
		// within 3s.
		let timeout = frame_support::sp_io::offchain::timestamp()
			.add(Duration::from_millis(FETCH_TIMEOUT_PERIOD));

		let pending = request
			.deadline(timeout) // Setting the timeout time
			.send() // Sending the request out by the host
			.map_err(|e| {
				println!("{:?}", e);
				// <Error<T>>::HttpFetchingError
			})
			.unwrap();

		// By default, the http request is async from the runtime perspective. So we are asking
		// the   runtime to wait here
		// The returning value here is a `Result` of `Result`, so we are unwrapping it twice by
		// two `?`   ref: https://docs.substrate.io/rustdocs/latest/sp_runtime/offchain/http/struct.PendingRequest.html#method.try_wait
		let response = pending
			.try_wait(timeout)
			.map_err(|e| {
				log::error!("{:?}", e);
				// <Error<T>>::HttpFetchingError
			})
			.unwrap()
			.map_err(|e| {
				log::error!("{:?}", e);
				// <Error<T>>::HttpFetchingError
			})
			.unwrap();

		if response.code != 200 {
			log::error!("Unexpected http request status code: {}", response.code);
			// return Err(<Error<T>>::HttpFetchingError)
		}

		let resp_bytes = response.body().collect::<Vec<u8>>();
		let resp_str = std::str::from_utf8(&resp_bytes).unwrap(); //.map_err(|_| <Error<T>>::DeserializeToStrError)?;
		log::info!("fetch_n_parse: {}", resp_str);

		// Deserializing JSON to struct, thanks to `serde` and `serde_derive`
		let info: Option<LightClientBlockView> = serde_json::from_str(&resp_str).unwrap(); //.map_err(|_| <Error<T>>::DeserializeToObjError)?;
		                                                                           // Ok(info)
	}
}

#[cfg(test)]
mod tests {
	use crate::mock::new_test_ext;

	use super::*;

	#[test]
	fn test_serialize_correctly() {
		let request = NearRpcRequestParams::NextBlock {
			last_block_hash: "2rs9o3B6nAQ3pEfVcBQdLnBqZrfpVuZJeKC8FpTshhua".to_string(),
		};
		let json_rpc: JsonRpc = request.into();
		assert_eq!(json_rpc.jsonrpc, "2.0");
		assert_eq!(json_rpc.method, "next_light_client_block");
		let req = serde_json::to_string(&json_rpc).unwrap();
		println!("{}", req);
		assert_eq!(
			req,
			r#"{"jsonrpc":"2.0","method":"next_light_client_block","params":{"last_block_hash":"2rs9o3B6nAQ3pEfVcBQdLnBqZrfpVuZJeKC8FpTshhua"},"id":"pallet-near"}"#
		)
	}

	#[test]
	fn test_give_it_a_bash() {
		new_test_ext().execute_with(|| {
			let r =
				NearRpcClient.fetch_latest_header("2rs9o3B6nAQ3pEfVcBQdLnBqZrfpVuZJeKC8FpTshhua");
		});
	}
}
