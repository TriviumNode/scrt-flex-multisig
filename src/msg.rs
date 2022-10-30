use cosmwasm_std::{Addr, Binary, CosmosMsg, Uint128};
use schemars::JsonSchema;
use secret_toolkit::{permit::Permit, serialization::Json, utils::HandleCallback};
use serde::{Deserialize, Serialize};

use crate::state::ExtActionProposition;

pub const BLOCK_SIZE: usize = 256;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub time_limit: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    TransferVotes {
        recipient: String,
        num_votes: Uint128,
    },
    ProposeAction {
        prop_msg: CosmosMsg,
    },
    VoteAction {
        action_prop: Uint128,
    },
    CreateViewingKey {
        entropy: String,
    },
    SetViewingKey {
        key: String,
    },
    RevokePermit {
        permit_name: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    QueryEx {},
    AllActions {
        start_page: Option<u32>,
        page_size: Option<u32>,
        viewer: String,
        key: String,
    },
    QueryAction {
        id: Uint128,
        viewer: String,
        key: String,
    },
    AllCompletedActions {
        start_page: Option<u32>,
        page_size: Option<u32>,
        viewer: String,
        key: String,
    },
    QueryCompletedAction {
        id: Uint128,
        viewer: String,
        key: String,
    },
    WithPermit {
        permit: Permit,
        query: QueryWithPermit,
    },
}

impl QueryMsg {
    pub fn get_validation_params(&self) -> (String, String) {
        match self {
            Self::AllActions { viewer, key, .. } => (viewer.to_string(), key.clone()),
            Self::QueryAction { viewer, key, .. } => (viewer.to_string(), key.clone()),
            Self::AllCompletedActions { viewer, key, .. } => (viewer.to_string(), key.clone()),
            Self::QueryCompletedAction { viewer, key, .. } => (viewer.to_string(), key.clone()),
            _ => panic!("This query type does not require authentication"),
        }
    }
}

/// queries using permits instead of viewing keys
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryWithPermit {
    AllActions {
        start_page: Option<u32>,
        page_size: Option<u32>,
    },
    QueryAction {
        id: Uint128,
    },
    AllCompletedActions {
        start_page: Option<u32>,
        page_size: Option<u32>,
    },
    QueryCompletedAction {
        id: Uint128,
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    AllActions {
        actions: Vec<(Uint128, ExtActionProposition)>,
    },
    QueryAction {
        action: ExtActionProposition,
    },
    ViewingKeyError {
        error: String,
    },
}

/// code hash and address of a contract
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Clone, Debug)]
pub struct ContractInfo {
    /// contract's code hash string
    pub code_hash: String,
    /// contract's address
    pub address: String,
}
