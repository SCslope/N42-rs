use alloy_primitives::BlockNumber;
use reth_revm::cached::CachedReads;
use serde::{Deserialize, Serialize};
#[derive(Serialize,Clone,Deserialize)]
pub struct UnverifiedBlock{
    pub blocknumber:BlockNumber,
    pub block:CachedReads,
}