use std::{
    any::{Any, TypeId},
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};

use ipis::{
    core::anyhow::{anyhow, bail, Result},
    resource::Resource,
    tokio::sync::Mutex,
};

#[derive(Default)]
pub struct ResourceRoot {
    map: Mutex<HashMap<TypeId, Arc<dyn Any + Send + Sync + 'static>>>,
}

impl ResourceRoot {
    pub async fn get<T>(&self) -> Result<Arc<T>>
    where
        T: Resource + Send + Sync + 'static,
    {
        self.map
            .lock()
            .await
            .get(&TypeId::of::<T>())
            .cloned()
            .and_then(|value| Arc::downcast(value).ok())
            .ok_or_else(|| anyhow!("failed to find the resource"))
    }

    pub async fn put<T>(&self, value: T) -> Result<()>
    where
        T: Resource + Send + Sync + 'static,
    {
        match self.map.lock().await.entry(TypeId::of::<T>()) {
            Entry::Vacant(e) => {
                e.insert(Arc::new(value));
                Ok(())
            }
            Entry::Occupied(_) => bail!("duplicated resource manager"),
        }
    }
}
