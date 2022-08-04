use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};

use ipis::{
    async_trait::async_trait, core::anyhow::Result, resource::Resource, rkyv::AlignedVec,
    tokio::sync::Mutex,
};
use ipwis_modules_task_common_wasi::interrupt_id::InterruptId;

use crate::{
    interrupt_handler::InterruptHandler, memory::IpwisMemory, task_manager::IpwisTaskManager,
};

pub(crate) type IpwisInterruptHandler = Arc<Mutex<Box<dyn InterruptHandler>>>;

pub struct InterruptHandlerState {
    manager: Arc<IpwisTaskManager>,
    map: HashMap<InterruptId, IpwisInterruptHandler>,
}

impl InterruptHandlerState {
    pub(crate) fn with_manager(manager: Arc<IpwisTaskManager>) -> Self {
        Self {
            manager,
            map: Default::default(),
        }
    }
}

impl InterruptHandlerState {
    pub async fn get(&mut self, handler: InterruptId) -> Result<IpwisInterruptHandler> {
        // load interrupt module
        if let Entry::Vacant(e) = self.map.entry(handler) {
            e.insert(self.manager.interrupt_manager.get(&handler).await?);
        }
        Ok(self.map.get_mut(&handler).unwrap().clone())
    }

    pub async unsafe fn syscall_raw(
        &mut self,
        memory: &mut IpwisMemory,
        handler: InterruptId,
        inputs: &[u8],
    ) -> Result<AlignedVec> {
        // load interrupt module
        if let Entry::Vacant(e) = self.map.entry(handler) {
            e.insert(self.manager.interrupt_manager.get(&handler).await?);
        }
        let handler = self.map.get(&handler).unwrap();

        handler.lock().await.handle_raw(memory, inputs).await
    }
}

#[async_trait]
impl Resource for InterruptHandlerState {
    async fn release(&mut self) -> Result<()> {
        for (_, handler) in self.map.drain() {
            handler.lock().await.release().await?;
        }
        Ok(())
    }
}
