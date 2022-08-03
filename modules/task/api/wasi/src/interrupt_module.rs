use ipis::{async_trait::async_trait, core::anyhow::Result};
use ipwis_modules_task_common_wasi::interrupt_id::InterruptId;

use crate::{
    interrupt_handler::InterruptHandler,
    memory::{IpwisMemory, Memory},
};

#[async_trait]
pub trait InterruptModule<M = IpwisMemory>
where
    Self: Send + Sync + 'static,
    M: Memory,
{
    fn id(&self) -> InterruptId;

    async fn spawn_handler(&self) -> Result<Box<dyn InterruptHandler<M>>>;
}
