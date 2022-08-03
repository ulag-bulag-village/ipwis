use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};

use ipis::{
    core::anyhow::{anyhow, bail, Result},
    tokio::sync::Mutex,
};
use ipwis_modules_task_common_wasi::interrupt_id::InterruptId;

use crate::{interrupt_handler_state::IpwisInterruptHandler, interrupt_module::InterruptModule};

type IpwisInterruptModule = Box<dyn InterruptModule>;

#[derive(Default)]
pub struct InterruptManager {
    map: Mutex<HashMap<InterruptId, IpwisInterruptModule>>,
}

impl InterruptManager {
    pub async fn get(&self, id: &InterruptId) -> Result<IpwisInterruptHandler> {
        let map = self.map.lock().await;
        let module = map
            .get(id)
            .ok_or_else(|| anyhow!("failed to find the interrupt module: {id}"))?;

        module.spawn_handler().await.map(Mutex::new).map(Arc::new)
    }

    pub async fn put<T>(&self, module: T) -> Result<()>
    where
        T: InterruptModule,
    {
        match self.map.lock().await.entry(module.id()) {
            Entry::Vacant(e) => {
                e.insert(Box::new(module));
                Ok(())
            }
            Entry::Occupied(e) => bail!("duplicated interrupt module: {}", e.key()),
        }
    }
}
