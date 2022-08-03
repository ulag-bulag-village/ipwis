use std::any::Any;

use ipis::{async_trait::async_trait, core::anyhow::Result, resource::Resource};
use rkyv::AlignedVec;

use crate::memory::{IpwisMemory, Memory};

#[async_trait]
pub trait InterruptHandler<M = IpwisMemory>
where
    Self: Any + Resource + Send + Sync,
    M: Memory,
{
    async unsafe fn handle_raw(&mut self, memory: &mut M, inputs: &[u8]) -> Result<AlignedVec>;
}
