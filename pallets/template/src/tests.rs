use crate::{
	mock::*,
	near::{
		client::{JsonRpcResult, NearRpcResult},
		views::{LightClientBlockView, ValidatorStakeView, ValidatorStakeViewScaleHax},
		LightClientState,
	},
	Error, Event,
};
use frame_support::{assert_noop, assert_ok};
