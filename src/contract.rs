use cosmwasm_std::{
    entry_point, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, Storage,
    Uint128,
};

use secret_toolkit::permit::{validate, Permit, RevokedPermits, TokenPermissions};

use secret_toolkit::viewing_key::{ViewingKey, ViewingKeyStore};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryAnswer, QueryMsg, QueryWithPermit};
use crate::state::{
    Config, ExtActionProposition, COMPLETED_ACTIONS, CONFIG_KEY, PENDING_ACTIONS,
    PREFIX_REVOKED_PERMITS, STAKEHOLDERS, TOT_PROPS, TOT_VOTES,
};

pub const DEFAULT_PAGE_SIZE: u32 = 200;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        contract_address: env.contract.address,
        prop_time_limit: msg.time_limit,
    };

    // Save data to storage
    CONFIG_KEY.save(deps.storage, &config)?;
    TOT_PROPS.save(deps.storage, &Uint128::from(0_u128))?;

    Ok(Response::new())
}

//-------------------------------------------- HANDLES ---------------------------------

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateViewingKey { entropy } => try_create_key(deps, env, info, entropy),
        ExecuteMsg::SetViewingKey { key, .. } => try_set_key(deps, info, &key),
        ExecuteMsg::TransferVotes {
            recipient,
            num_votes,
        } => transfer_votes(deps, env, info, recipient, num_votes),
        ExecuteMsg::ProposeAction { prop_msg } => propose_new_action(deps, env, info, prop_msg),
        ExecuteMsg::VoteAction { action_prop } => vote_new_action(deps, env, info, action_prop),
        ExecuteMsg::RevokePermit { permit_name } => revoke_permit(deps, env, info, permit_name),
    }
}

/// Returns Result<Response, ContractError>
///
/// moves votes from one address to another
///
/// # Arguments
///
/// * `deps`    - DepsMut containing all the contract's external dependencies
/// * `env`     - Env of contract's environment
/// * `info`    - Carries the info of who sent the message and how much native funds were sent along
fn transfer_votes(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    num_votes: Uint128,
) -> Result<Response, ContractError> {
    if !STAKEHOLDERS.contains(deps.storage, &info.sender.to_string()) {
        return Err(ContractError::CustomError {
            val: "You do not have a share in this contract".to_string(),
        });
    }

    let sender_votes = STAKEHOLDERS
        .get(deps.storage, &info.sender.to_string())
        .unwrap();
    if num_votes > sender_votes {
        return Err(ContractError::CustomError {
            val: "You cannot transfer a larger share than you posess".to_string(),
        });
    } else {
        STAKEHOLDERS.insert(
            deps.storage,
            &info.sender.to_string(),
            &(sender_votes - num_votes),
        )?;
    }

    if STAKEHOLDERS.contains(deps.storage, &recipient) {
        let reciever_votes = STAKEHOLDERS.get(deps.storage, &recipient).unwrap();
        STAKEHOLDERS.insert(
            deps.storage,
            &info.sender.to_string(),
            &(reciever_votes + num_votes),
        )?;
    } else {
        STAKEHOLDERS.insert(deps.storage, &info.sender.to_string(), &num_votes)?;
    }

    Ok(Response::new())
}

/// Returns Result<Response, ContractError>
///
/// adds a votable prop to place action
///
/// # Arguments
///
/// * `deps`    - DepsMut containing all the contract's external dependencies
/// * `env`     - Env of contract's environment
/// * `info`    - Carries the info of who sent the message and how much native funds were sent along
fn propose_new_action(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    prop_msg: CosmosMsg,
) -> Result<Response, ContractError> {
    if !STAKEHOLDERS.contains(deps.storage, &info.sender.to_string()) {
        return Err(ContractError::CustomError {
            val: "You do not have a share in this contract".to_string(),
        });
    }
    let new_prop = ExtActionProposition {
        confirmed_votes: Uint128::from(0_u128),
        proposed_at: env.block.time,
        cosmos_msg: prop_msg,
    };

    let prop_num = TOT_PROPS.load(deps.storage)?;
    TOT_PROPS.save(deps.storage, &(prop_num + Uint128::from(1_u128)))?;
    PENDING_ACTIONS.insert(deps.storage, &(prop_num + Uint128::from(1_u128)), &new_prop)?;

    Ok(Response::new())
}

/// Returns Result<Response, ContractError>
///
/// votes in favor of new action
///
/// # Arguments
///
/// * `deps`    - DepsMut containing all the contract's external dependencies
/// * `env`     - Env of contract's environment
/// * `info`    - Carries the info of who sent the message and how much native funds were sent along
fn vote_new_action(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    action_prop: Uint128,
) -> Result<Response, ContractError> {
    if !PENDING_ACTIONS.contains(deps.storage, &action_prop) {
        return Err(ContractError::CustomError {
            val: "This propostion does not exist".to_string(),
        });
    } else if !STAKEHOLDERS.contains(deps.storage, &info.sender.to_string()) {
        return Err(ContractError::CustomError {
            val: "You do not have a share in this contract".to_string(),
        });
    }

    let mut prop = PENDING_ACTIONS.get(deps.storage, &action_prop).unwrap();
    let tot_votes = TOT_VOTES.load(deps.storage)?;

    // Check if expiration time has passed
    let config = CONFIG_KEY.load(deps.storage)?;
    if prop.proposed_at.plus_seconds(config.prop_time_limit) > env.block.time {
        PENDING_ACTIONS.remove(deps.storage, &action_prop)?;
        return Ok(Response::new().add_attribute("Removed Prop", "Timed Out"));
    }

    if prop.confirmed_votes.u128() < (tot_votes.u128() / 2) {
        let votes = STAKEHOLDERS
            .get(deps.storage, &info.sender.to_string())
            .unwrap();
        prop.confirmed_votes += votes;
        PENDING_ACTIONS.insert(deps.storage, &action_prop, &prop)?;
    } else {
        COMPLETED_ACTIONS.insert(deps.storage, &action_prop, &prop)?;
        PENDING_ACTIONS.remove(deps.storage, &action_prop)?;
        return Ok(Response::new().add_message(prop.cosmos_msg));
    }

    Ok(Response::new())
}

/// Returns Result<Response, ContractError>
///
/// votes in favor of new action
///
/// # Arguments
///
/// * `deps`    - DepsMut containing all the contract's external dependencies
/// * `env`     - Env of contract's environment
/// * `info`    - Carries the info of who sent the message and how much native funds were sent along
fn purge_expired_actions(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    start_page: Option<u32>,
    page_size: Option<u32>,
) -> Result<Response, ContractError> {
    if !STAKEHOLDERS.contains(deps.storage, &info.sender.to_string()) {
        return Err(ContractError::CustomError {
            val: "You do not have a share in this contract".to_string(),
        });
    }
    let config = CONFIG_KEY.load(deps.storage)?;

    // Check for defaults
    let start = start_page.unwrap_or(0);
    let size = page_size.unwrap_or(DEFAULT_PAGE_SIZE);

    let paginated_action_iter = PENDING_ACTIONS.paging(deps.storage, start, size)?;

    //let init_len = PENDING_ACTIONS.get_len(deps.storage)?;

    // Loop through Issuers and cnvert to ExportIssuer
    for action in paginated_action_iter {
        // Check if expiration time has passed
        if action.1.proposed_at.plus_seconds(config.prop_time_limit) > env.block.time {
            PENDING_ACTIONS.remove(deps.storage, &action.0)?;
        }
    }

    //let removed_count = init_len - PENDING_ACTIONS.get_len(deps.storage)?;

    Ok(Response::new())
}

/// Returns Result<Response, ContractError>
///
/// create a viewing key
///
/// # Arguments
///
/// * `deps`    - DepsMut containing all the contract's external dependencies
/// * `env`     - Env of contract's environment
/// * `info`    - Carries the info of who sent the message and how much native funds were sent along
/// * `entropy` - string to be used as an entropy source for randomization
fn try_create_key(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    entropy: String,
) -> Result<Response, ContractError> {
    let key = ViewingKey::create(
        deps.storage,
        &info,
        &env,
        info.sender.as_str(),
        entropy.as_bytes(),
    );

    Ok(Response::new().add_attribute("viewing_key", key))
}

/// Returns Result<Response, ContractError>
///
/// sets the viewing key
///
/// # Arguments
///
/// * `deps` - DepsMut containing all the contract's external dependencies
/// * `info` - Carries the info of who sent the message and how much native funds were sent along
/// * `key`  - string slice to be used as the viewing key
fn try_set_key(deps: DepsMut, info: MessageInfo, key: &str) -> Result<Response, ContractError> {
    ViewingKey::set(deps.storage, info.sender.as_str(), key);

    Ok(Response::new().add_attribute("viewing_key", key))
}

fn revoke_permit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    permit_name: String,
) -> Result<Response, ContractError> {
    RevokedPermits::revoke_permit(
        deps.storage,
        PREFIX_REVOKED_PERMITS,
        info.sender.as_ref(),
        &permit_name,
    );

    Ok(Response::new())
}

// ---------------------------------------- QUERIES --------------------------------------

#[entry_point]
pub fn query(deps: Deps, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::WithPermit { permit, query } => permit_queries(deps, permit, query),
        _ => viewing_keys_queries(deps, msg),
    }
}

/// Returns QueryResult from validating a permit and then using its creator's address when
/// performing the specified query
///
/// # Arguments
///
/// * `deps` - a reference to Extern containing all the contract's external dependencies
/// * `permit` - the permit used to authentic the query
/// * `query` - the query to perform
fn permit_queries(
    deps: Deps,
    permit: Permit,
    query: QueryWithPermit,
) -> Result<Binary, ContractError> {
    // Validate permit content
    let config = CONFIG_KEY.load(deps.storage)?;

    let viewer = validate(
        deps,
        PREFIX_REVOKED_PERMITS,
        &permit,
        config.contract_address.to_string(),
        None,
    )?;

    // Permit validated! We can now execute the query.
    match query {
        QueryWithPermit::AllActions {
            start_page,
            page_size,
        } => {
            if !permit.check_permission(&TokenPermissions::Balance) {
                return Err(ContractError::Unauthorized {});
            }

            query_all_actions(deps, start_page, page_size, viewer)
        }
        QueryWithPermit::QueryAction { id } => {
            if !permit.check_permission(&TokenPermissions::Balance) {
                return Err(ContractError::Unauthorized {});
            }

            query_action(deps, id, viewer)
        }
        QueryWithPermit::AllCompletedActions {
            start_page,
            page_size,
        } => query_all_completed_actions(deps, start_page, page_size, viewer),
        QueryWithPermit::QueryCompletedAction { id } => query_completed_action(deps, id, viewer),
    }
}

pub fn viewing_keys_queries(deps: Deps, msg: QueryMsg) -> Result<Binary, ContractError> {
    let (address, key) = msg.get_validation_params();

    if !is_key_valid(deps.storage, &address, key) {
        Err(ContractError::Unauthorized {})
    } else {
        match msg {
            // Base
            QueryMsg::AllActions {
                viewer,
                key: _,
                start_page,
                page_size,
            } => query_all_actions(deps, start_page, page_size, viewer),
            QueryMsg::QueryAction { viewer, key: _, id } => query_action(deps, id, viewer),
            QueryMsg::AllCompletedActions {
                viewer,
                key: _,
                start_page,
                page_size,
            } => query_all_completed_actions(deps, start_page, page_size, viewer),
            QueryMsg::QueryCompletedAction { viewer, key: _, id } => {
                query_completed_action(deps, id, viewer)
            }

            _ => panic!("This query type does not require authentication"),
        }
    }
}

fn query_all_actions(
    deps: Deps,
    start_page: Option<u32>,
    page_size: Option<u32>,
    viewer: String,
) -> Result<Binary, ContractError> {
    if !STAKEHOLDERS.contains(deps.storage, &viewer) {
        return Err(ContractError::CustomError {
            val: "You do not have a share in this contract".to_string(),
        });
    }

    // Check for defaults
    let start = start_page.unwrap_or(0);
    let size = page_size.unwrap_or(DEFAULT_PAGE_SIZE);

    // Prep empty List of Listing Data for response
    let mut action_list: Vec<(Uint128, ExtActionProposition)> = vec![];

    let mut paginated_action_iter = PENDING_ACTIONS
        .iter(deps.storage)?
        .skip((start as usize) * (size as usize))
        .take(size as usize);

    // Loop through Issuers and cnvert to ExportIssuer
    loop {
        let may_next_action = paginated_action_iter.next();
        if let Some(element) = may_next_action {
            let listing_pair = element?;

            action_list.push(listing_pair);
        } else {
            break;
        }
    }

    Ok(to_binary(&QueryAnswer::AllActions {
        actions: action_list,
    })?)
}

fn query_action(deps: Deps, id: Uint128, viewer: String) -> Result<Binary, ContractError> {
    if !STAKEHOLDERS.contains(deps.storage, &viewer) {
        return Err(ContractError::CustomError {
            val: "You do not have a share in this contract".to_string(),
        });
    }

    if !PENDING_ACTIONS.contains(deps.storage, &id) {
        return Err(ContractError::CustomError {
            val: "This ID is not linked to a pending action".to_string(),
        });
    }

    let action = PENDING_ACTIONS.get(deps.storage, &id).unwrap();
    Ok(to_binary(&QueryAnswer::QueryAction { action })?)
}

fn query_all_completed_actions(
    deps: Deps,
    start_page: Option<u32>,
    page_size: Option<u32>,
    viewer: String,
) -> Result<Binary, ContractError> {
    if !STAKEHOLDERS.contains(deps.storage, &viewer) {
        return Err(ContractError::CustomError {
            val: "You do not have a share in this contract".to_string(),
        });
    }

    // Check for defaults
    let start = start_page.unwrap_or(0);
    let size = page_size.unwrap_or(DEFAULT_PAGE_SIZE);

    // Prep empty List of Listing Data for response
    let mut action_list: Vec<(Uint128, ExtActionProposition)> = vec![];

    let mut paginated_action_iter = COMPLETED_ACTIONS
        .iter(deps.storage)?
        .skip((start as usize) * (size as usize))
        .take(size as usize);

    // Loop through Issuers and cnvert to ExportIssuer
    loop {
        let may_next_action = paginated_action_iter.next();
        if let Some(element) = may_next_action {
            let listing_pair = element?;

            action_list.push(listing_pair);
        } else {
            break;
        }
    }

    Ok(to_binary(&QueryAnswer::AllActions {
        actions: action_list,
    })?)
}

fn query_completed_action(
    deps: Deps,
    id: Uint128,
    viewer: String,
) -> Result<Binary, ContractError> {
    if !STAKEHOLDERS.contains(deps.storage, &viewer) {
        return Err(ContractError::CustomError {
            val: "You do not have a share in this contract".to_string(),
        });
    }

    if !COMPLETED_ACTIONS.contains(deps.storage, &id) {
        return Err(ContractError::CustomError {
            val: "This ID is not linked to a pending action".to_string(),
        });
    }

    let action = COMPLETED_ACTIONS.get(deps.storage, &id).unwrap();
    Ok(to_binary(&QueryAnswer::QueryAction { action })?)
}

//----------------------------------------- Helper functions----------------------------------

/// Returns bool result of validating an address' viewing key
///
/// # Arguments
///
/// * `storage`     - a reference to the contract's storage
/// * `account`     - a reference to the str whose key should be validated
/// * `viewing_key` - String key used for authentication
fn is_key_valid(storage: &dyn Storage, account: &str, viewing_key: String) -> bool {
    ViewingKey::check(storage, account, &viewing_key).is_ok()
}
