use std::sync::Arc;

use ipis::{
    async_trait::async_trait, core::anyhow::Result, resource::Resource, tokio::sync::Mutex,
};
use ipwis_modules_task_api::task_state::TaskState;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

use crate::{interrupt_handler_state::InterruptHandlerState, task_manager::IpwisTaskManager};

pub struct IpwisTaskCtx {
    pub wasi: WasiCtx,
    pub state: Arc<Mutex<TaskState<IpwisTaskManager>>>,
    pub interrupt_handler_state: InterruptHandlerState,
}

impl IpwisTaskCtx {
    pub fn try_new(
        manager: Arc<IpwisTaskManager>,
        state: Arc<Mutex<TaskState<IpwisTaskManager>>>,
    ) -> Result<Self> {
        Ok(Self {
            // create a WASI context and put it in a Store; all instances in the store
            // share this context. `WasiCtxBuilder` provides a number of ways to
            // configure what the target program will have access to.
            wasi: WasiCtxBuilder::new()
                .inherit_stdio()
                .inherit_args()?
                .build(),
            state,
            interrupt_handler_state: InterruptHandlerState::with_manager(manager),
        })
    }
}

#[async_trait]
impl Resource for IpwisTaskCtx {
    async fn release(&mut self) -> Result<()> {
        self.interrupt_handler_state.release().await?;
        Ok(())
    }
}
