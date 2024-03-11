use std::fmt::Display;

use chain_data::{next_opt, parse_chain_data, ChainDataError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DealStatus {
    // Deal does have enough funds to pay for the workers
    InsufficientFunds = 0,
    Active,
    // Deal is stopped
    Ended,
    // Deal has a balance and waiting for workers
    NotEnoughWorkers,
    // Deal has balance less than the minimal balance. Min balance: 2 * targetWorkers * pricePerWorkerEpoch
    SmallBalance,
}

impl DealStatus {
    pub fn signature() -> Vec<ethabi::ParamType> {
        vec![ethabi::ParamType::Uint(8)]
    }
    pub fn from_num(num: u8) -> Option<Self> {
        match num {
            0 => Some(DealStatus::InsufficientFunds),
            1 => Some(DealStatus::Active),
            2 => Some(DealStatus::Ended),
            3 => Some(DealStatus::NotEnoughWorkers),
            4 => Some(DealStatus::SmallBalance),
            _ => None,
        }
    }

    pub fn from_token(token: ethabi::Token) -> Option<Self> {
        token
            .into_uint()
            .and_then(|u| Self::from_num(u.as_u64() as u8))
    }

    pub fn from(data: &str) -> Result<Self, ChainDataError> {
        let mut tokens = parse_chain_data(data, &Self::signature())?.into_iter();
        next_opt(&mut tokens, "deal_status", Self::from_token)
    }
}

impl Display for DealStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            DealStatus::Active => "Active",
            DealStatus::Ended => "Ended",
            DealStatus::NotEnoughWorkers => "NotEnoughWorkers",
            DealStatus::SmallBalance => "SmallBalance",
            DealStatus::InsufficientFunds => "InsufficientFunds",
        }
        .to_string();
        write!(f, "{}", str)
    }
}
