use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::Item;
use cw_storage_plus::Map;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Payment {
    pub sender: Addr,
    pub receiver: Addr,
    pub amount: i32,
    pub amount_in_coins: Coin,
    pub deadline: i32,
    pub claimed: bool,
    pub reverted: bool,
    pub payment_id: i32
}

pub const SENT_PAYMENTS: Map<(Addr, i32), Payment> = Map::new("sent_payments");
pub const RECEIVED_PAYMENTS: Map<(Addr, i32), Payment> = Map::new("received_payments");
pub const PAYMENTS: Map<i32, Payment> = Map::new("payments");
pub const CURR_PAYMENT_ID: Item<i32> = Item::new("curr_payment_id");

// let mut sent_payments: HashMap<Address, HashMap<u256, Payment>> = HashMap::new();
// let mut received_payments: HashMap<Address, HashMap<u256, Payment>> = HashMap::new();
// let mut payments: Vec<Payment> = Vec::new();
// let mut curr_payment_id: u256 = 0;
