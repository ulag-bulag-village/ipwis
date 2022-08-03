use std::collections::HashMap;

use bytecheck::CheckBytes;
use ipis::{
    async_trait::async_trait,
    core::anyhow::{anyhow, Result},
    resource::Resource,
};
use rkyv::{Archive, Deserialize, Serialize};

pub struct ResourceStore<R> {
    map: HashMap<ResourceId, R>,
    seed: ResourceId,
}

impl<R> Default for ResourceStore<R> {
    fn default() -> Self {
        Self {
            map: Default::default(),
            seed: ResourceId::zero(),
        }
    }
}

impl<R> ResourceStore<R> {
    pub fn get(&self, id: &ResourceId) -> Result<&R> {
        self.map
            .get(id)
            .ok_or_else(|| anyhow!("failed to find a resource: {id:x}"))
    }

    pub fn get_mut(&mut self, id: &ResourceId) -> Result<&mut R> {
        self.map
            .get_mut(id)
            .ok_or_else(|| anyhow!("failed to find a resource: {id:x}"))
    }

    pub fn put(&mut self, value: R) -> ResourceId {
        let id = self.seed.next();
        self.map.insert(id, value);
        id
    }

    pub async fn release_one(&mut self, id: &ResourceId) -> Result<()>
    where
        R: Resource + Send,
    {
        match self.map.remove(id) {
            Some(mut value) => {
                value.release().await?;
                Ok(())
            }
            None => Ok(()),
        }
    }
}

#[async_trait]
impl<R> Resource for ResourceStore<R>
where
    R: Resource + Send,
{
    async fn release(&mut self) -> Result<()> {
        for (_, mut value) in self.map.drain() {
            value.release().await?;
        }
        Ok(())
    }
}

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Archive, Serialize, Deserialize,
)]
#[archive_attr(derive(CheckBytes, Copy, Clone, Debug))]
pub struct ResourceId(u64);

impl ::core::fmt::LowerHex for ResourceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        ::core::fmt::LowerHex::fmt(&self.0, f)
    }
}

impl ResourceId {
    fn zero() -> Self {
        Self(Default::default())
    }

    fn next(&mut self) -> Self {
        let current = *self;
        self.0 += 1;
        current
    }
}
