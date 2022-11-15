use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use secret_toolkit::{
    serialization::Json,
    storage::{Item, Keymap},
};

use cosmwasm_std::{Addr, CosmosMsg, Timestamp, Uint128};

/// Map of permission holders and number of votes
pub static STAKEHOLDERS: Keymap<String, Uint128> = Keymap::new(b"stakeholders");
/// Total number of votes availible
pub static TOT_VOTES: Item<Uint128> = Item::new(b"votes");
/// Current prop number
pub static TOT_PROPS: Item<Uint128> = Item::new(b"props");

/// Map of pending actions to vote on
pub static PENDING_ACTIONS: Keymap<Uint128, ExtActionProposition, Json> =
    Keymap::new(b"actionprop");
/// Map of pending stake adjustments
pub static COMPLETED_ACTIONS: Keymap<Uint128, ExtActionProposition> = Keymap::new(b"stakeprop");

// Record of whether an address voted. Must be used with a suffix of the prop ID
pub static VOTE_RECORD: Keymap<String, bool> = Keymap::new(b"stakeprop");

/// Basic configuration struct
pub static CONFIG_KEY: Item<Config> = Item::new(b"config");
/// Revoked permits prefix key
pub const PREFIX_REVOKED_PERMITS: &str = "revoked_permits";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub contract_address: Addr,
    pub prop_time_limit: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct StakeProposition {
    // Votes supporting the proposal
    pub confirmed_votes: Uint128,
    // Time proposition was made
    pub proposed_at: Timestamp,
    // Vote stake recipient
    pub recipient: String,
    // proposed vote amount
    pub num_votes: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ExtActionProposition {
    // Votes supporting the proposal
    pub confirmed_votes: Uint128,
    // Time proposition was made
    pub proposed_at: Timestamp,
    pub cosmos_msg: CosmosMsg,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Transferer {
    // holder address
    pub addr: String,
    //votes being transfered from holder
    pub amount: Uint128,
}
