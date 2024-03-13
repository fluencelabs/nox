use std::fmt::Display;

use chain_data::{next_opt, parse_chain_data, ChainDataError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitmentStatus {
    Inactive = 0,
    Active,
    // WaitDelegation - before collateral is deposited.
    WaitDelegation,
    // Status is WaitStart - means collateral deposited, and epoch should be proceed before Active.
    WaitStart,
    Failed,
    Removed,
}

impl CommitmentStatus {
    pub fn signature() -> Vec<ethabi::ParamType> {
        vec![ethabi::ParamType::Uint(8)]
    }
    pub fn from_num(num: u8) -> Option<Self> {
        match num {
            0 => Some(CommitmentStatus::Active),
            1 => Some(CommitmentStatus::WaitDelegation),
            2 => Some(CommitmentStatus::WaitStart),
            3 => Some(CommitmentStatus::Inactive),
            4 => Some(CommitmentStatus::Failed),
            5 => Some(CommitmentStatus::Removed),
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
        next_opt(&mut tokens, "commitment_status", Self::from_token)
    }
}

impl Display for CommitmentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            CommitmentStatus::Active => "Active",
            CommitmentStatus::WaitDelegation => "WaitDelegation",
            CommitmentStatus::WaitStart => "WaitStart",
            CommitmentStatus::Inactive => "Inactive",
            CommitmentStatus::Failed => "Failed",
            CommitmentStatus::Removed => "Removed",
        }
        .to_string();
        write!(f, "{}", str)
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn decode_commitment_status() {
        let data = "0x0000000000000000000000000000000000000000000000000000000000000001";
        let status = super::CommitmentStatus::from(data);
        assert!(status.is_ok());
        let status = status.unwrap();
        assert_eq!(status, super::CommitmentStatus::WaitDelegation);
    }

    #[tokio::test]
    async fn decode_commitment_status_removed() {
        let data = "0x0000000000000000000000000000000000000000000000000000000000000005";
        let status = super::CommitmentStatus::from(data);
        assert!(status.is_ok());
        let status = status.unwrap();
        assert_eq!(status, super::CommitmentStatus::Removed);
    }
}
