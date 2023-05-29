#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Addr, Uint128, BankMsg, Coin};
use cosmwasm_storage::{singleton, Singleton, ReadonlySingleton};
use cw_storage_plus::Bound;
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{CountResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Payment, SENT_PAYMENTS, RECEIVED_PAYMENTS, PAYMENTS, CURR_PAYMENT_ID};
use std::convert::TryInto;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:try1";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CURR_PAYMENT_ID_KEY: &[u8] = b"curr_payment_id";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // let state = State {
    //     count: msg.count,
    //     owner: info.sender.clone(),
    // };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // STATE.save(deps.storage, &state)?;

    let mut singleton = Singleton::new(deps.storage, CURR_PAYMENT_ID_KEY);
    let curr_payment_id = 0;
    singleton.save(&curr_payment_id)?;
    
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("count", curr_payment_id.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Increment {} => try_increment(deps),
        ExecuteMsg::Reset { count } => try_reset(deps, info, count),
        ExecuteMsg::SendPayment {receiver, time_ahead} => try_send_payment(receiver, time_ahead, info, _env, deps),
        ExecuteMsg::ClaimPayment {payment_id} => try_claim_payment(payment_id, info, _env, deps),
        ExecuteMsg::RevertPayment {payment_id} => try_revert_payment(payment_id, info, _env, deps)
    }
}

pub fn try_revert_payment(payment_id: i32, info: MessageInfo, _env: Env, deps: DepsMut) -> Result<Response, ContractError> {

    let tx_sender = info.sender.clone();

    let payment_result = SENT_PAYMENTS.load(deps.storage, (tx_sender.clone(), payment_id));
    let mut payment: Payment = Payment {
        sender: tx_sender.clone(),
        receiver: tx_sender.clone(),
        amount: 0,
        amount_in_coins: Coin::new(0.try_into().unwrap(), "inj".to_owned()),
        deadline: 0,
        claimed: false,
        reverted: false,
        payment_id: 0
    };

    let mut no_err = true;
    let mut error_data = "".to_string();

    match payment_result {
        Ok(payment_) => {
            payment = payment_;
        }
        Err(err) => {
            no_err = false;
            error_data = err.to_string();
        }
    }

    assert!(no_err, error_data.clone());
    assert!(tx_sender == payment.sender, "Sender can only revert their own payments");
    assert!(!payment.claimed, "Payment has already been claimed");
    assert!(!payment.reverted, "Payment has already been reverted");

    let new_payment = Payment {
        sender: payment.sender.clone(),
        receiver: payment.receiver.clone(),
        amount: payment.amount,
        amount_in_coins: payment.amount_in_coins.clone(),
        deadline: payment.deadline,
        claimed: payment.claimed,
        reverted: true,
        payment_id: payment.payment_id
    };

    let sent_payments_key = (tx_sender.clone(), payment_id);
    let received_payments_key = (payment.receiver.clone(), payment_id);

    SENT_PAYMENTS.save(deps.storage, sent_payments_key, &new_payment).unwrap();
    RECEIVED_PAYMENTS.save(deps.storage, received_payments_key, &new_payment).unwrap();

    let send_message = BankMsg::Send {
        to_address: tx_sender.clone().to_string(),
        amount: vec![payment.amount_in_coins.clone()],
    };


    Ok(Response::new()
        .add_attribute("method", "try_revert_payment")
        .add_message(send_message)
        .add_attribute("coin", payment.amount_in_coins.clone().to_string())
    )


}

pub fn try_claim_payment(payment_id: i32, info: MessageInfo, _env: Env, deps: DepsMut) -> Result<Response, ContractError> {

    let tx_sender = info.sender.clone();

    let payment_result = RECEIVED_PAYMENTS.load(deps.storage, (tx_sender.clone(), payment_id));
    let mut payment: Payment = Payment {
        sender: tx_sender.clone(),
        receiver: tx_sender.clone(),
        amount: 0,
        deadline: 0,
        amount_in_coins: Coin::new(0.try_into().unwrap(), "inj".to_owned()),
        claimed: false,
        reverted: false,
        payment_id: 0
    };

    let mut no_err = true;
    let mut error_data = "".to_string();

    match payment_result {
        Ok(payment_) => {
            payment = payment_;
        }
        Err(err) => {
            no_err = false;
            error_data = err.to_string();
        }
    }

    assert!(no_err, error_data.clone());
    assert!(tx_sender == payment.receiver, "Receiver can only claim the payment");
    assert!(!payment.claimed, "Payment has already been claimed");
    assert!(!payment.reverted, "Payment has been reverted by the sender");
    assert!(_env.block.time.seconds() as i32 <= payment.deadline, "Payment deadline has passed!");

    let new_payment = Payment {
        sender: payment.sender.clone(),
        receiver: payment.receiver,
        amount: payment.amount,
        amount_in_coins: payment.amount_in_coins.clone(),
        deadline: payment.deadline,
        claimed: true,
        reverted: payment.reverted,
        payment_id: payment.payment_id
    };

    let sent_payments_key = (payment.sender, payment_id);
    let received_payments_key = (tx_sender.clone(), payment_id);

    SENT_PAYMENTS.save(deps.storage, sent_payments_key, &new_payment).unwrap();
    RECEIVED_PAYMENTS.save(deps.storage, received_payments_key, &new_payment).unwrap();

    let send_message = BankMsg::Send {
        to_address: tx_sender.clone().to_string(),
        amount: vec![payment.amount_in_coins.clone()]
    };


    Ok(Response::new()
        .add_attribute("method", "try_claim_payment")
        .add_message(send_message)
    )

}

pub fn try_send_payment(receiver: Addr, time_ahead: i32, info: MessageInfo, _env: Env, deps: DepsMut) -> Result<Response, ContractError> {

    let sender = info.sender.clone();
    let mut amount = 0;
    if info.funds.is_empty() {
        amount = 0
    } else {
        amount = info.funds[0].amount.u128().clone();
    }

    assert_ne!(sender, receiver, "Sender and receiver cannot be the same");

    // Check if the deadline is in the future
    let deadline = _env.block.time.plus_seconds((time_ahead as u64));
    assert!(deadline > _env.block.time, "Deadline must be in the future");

    // Check if the payment amount is greater than zero
    assert!(!info.funds.is_empty(), "Payment amount must be greater than zero");

    let mut singleton = Singleton::new(deps.storage, CURR_PAYMENT_ID_KEY);
    let mut payment_id_state: i32 = singleton.load()?;
    payment_id_state += 1; // Increment the state value
    singleton.save(&payment_id_state)?;

    let payment = Payment {
        sender: sender.clone(),
        receiver: receiver.clone(),
        amount: amount as i32,
        amount_in_coins: info.funds[0].clone(),
        deadline: deadline.seconds() as i32,
        claimed: false,
        reverted: false,
        payment_id: payment_id_state
    };

    let sent_payments_key = (sender.clone(), payment_id_state);
    let received_payments_key = (receiver.clone(), payment_id_state);

    SENT_PAYMENTS.save(deps.storage, sent_payments_key, &payment).unwrap();
    RECEIVED_PAYMENTS.save(deps.storage, received_payments_key, &payment).unwrap();


    Ok(Response::new()
        .add_attribute("method", "try_send_payment")
        .add_attribute("receiver", receiver)
        .add_attribute("time_ahead", time_ahead.to_string())
        .add_attribute("sender", sender)
        .add_attribute("amount", amount.to_string())
    )
}

pub fn try_increment(deps: DepsMut) -> Result<Response, ContractError> {
    // STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
    //     state.count += 1;
    //     Ok(state)
    // })?;

    Ok(Response::new().add_attribute("method", "try_increment"))
}

pub fn try_reset(deps: DepsMut, info: MessageInfo, count: i32) -> Result<Response, ContractError> {
    // STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
    //     if info.sender != state.owner {
    //         return Err(ContractError::Unauthorized {});
    //     }
    //     state.count = count;
    //     Ok(state)
    // })?;
    Ok(Response::new().add_attribute("method", "reset"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCount {} => to_binary(&query_count(deps)?),
        QueryMsg::GetSentPayments {sender} => {
            let result = get_sent_payments(sender, deps);
            to_binary(&result.map_err(|err| err.to_string()))
        },
        QueryMsg::GetReceivedPayments {receiver} => {
            let result = get_received_payments(receiver, deps);
            to_binary(&result.map_err(|err| err.to_string()))
        }
    }
}

fn get_received_payments(receiver: Addr, deps: Deps)-> StdResult<Vec<Payment>> {

        let mut singleton = ReadonlySingleton::new(deps.storage, CURR_PAYMENT_ID_KEY);
        let mut curr_payment_id: i32 = singleton.load()?;
        curr_payment_id += 1;

        let payments = RECEIVED_PAYMENTS.prefix(receiver.clone()).range(deps.storage, Some(Bound::inclusive(1)), Some(Bound::exclusive(curr_payment_id)), cosmwasm_std::Order::Ascending)
        .map(|item| item.map(|(_, payment)| payment))
        .collect::<Result<Vec<Payment>, _>>()?;
        Ok(payments)
}

fn get_sent_payments(sender: Addr, deps: Deps)-> StdResult<Vec<Payment>> {

        let mut singleton = ReadonlySingleton::new(deps.storage, CURR_PAYMENT_ID_KEY);
        let mut curr_payment_id: i32 = singleton.load()?;
        curr_payment_id += 1;

        let payments = SENT_PAYMENTS.prefix(sender.clone()).range(deps.storage, Some(Bound::inclusive(1)), Some(Bound::exclusive(curr_payment_id)), cosmwasm_std::Order::Ascending)
        .map(|item| item.map(|(_, payment)| payment))
        .collect::<Result<Vec<Payment>, _>>()?;
        Ok(payments)

}

fn query_count(deps: Deps) -> StdResult<CountResponse> {

    let mut singleton = ReadonlySingleton::new(deps.storage, CURR_PAYMENT_ID_KEY);
    let mut curr_payment_id: i32 = singleton.load()?;

    // let state = STATE.load(deps.storage)?;
    // Ok(CountResponse { count: state.count })
    Ok(CountResponse { count: curr_payment_id })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg { count: 17 };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(17, value.count);
    }

    #[test]
    fn increment() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg { count: 17 };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("anyone", &coins(2, "token"));
        let msg = ExecuteMsg::Increment {};
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // should increase counter by 1
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(18, value.count);
    }

    #[test]
    fn reset() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg { count: 17 };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let unauth_info = mock_info("anyone", &coins(2, "token"));
        let msg = ExecuteMsg::Reset { count: 5 };
        let res = execute(deps.as_mut(), mock_env(), unauth_info, msg);
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }

        // only the original creator can reset the counter
        let auth_info = mock_info("creator", &coins(2, "token"));
        let msg = ExecuteMsg::Reset { count: 5 };
        let _res = execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();

        // should now be 5
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(5, value.count);
    }
}
