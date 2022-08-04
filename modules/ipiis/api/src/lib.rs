#![allow(clippy::missing_safety_doc)]

use ipiis_api::common::Ipiis;
use ipis::{
    async_trait::async_trait,
    core::{anyhow::Result, signed::IsSigned},
    env::Infer,
    pin::PinnedInner,
    resource::Resource,
    rkyv::AlignedVec,
};
use ipwis_modules_core_common::resource_store::ResourceStore;
use ipwis_modules_ipiis_common::io;
use ipwis_modules_stream_api::{StreamHandler, StreamModule};
use ipwis_modules_task_api_wasi::{
    interrupt_handler::InterruptHandler, interrupt_module::InterruptModule, memory::IpwisMemory,
};
use ipwis_modules_task_common_wasi::interrupt_id::InterruptId;

#[derive(Copy, Clone, Debug, Default)]
pub struct IpiisModule;

#[async_trait]
impl InterruptModule for IpiisModule {
    fn id(&self) -> InterruptId {
        io::OpCode::ID
    }

    async fn spawn_handler(&self) -> Result<Box<dyn InterruptHandler>> {
        Ok(Box::new(IpiisHandler {
            map: Default::default(),
        }))
    }
}

pub struct IpiisHandler {
    map: ResourceStore<::ipiis_api::client::IpiisClient>,
}

#[async_trait]
impl InterruptHandler for IpiisHandler {
    async unsafe fn handle_raw(
        &mut self,
        memory: &mut IpwisMemory,
        inputs: &[u8],
    ) -> Result<AlignedVec> {
        match PinnedInner::deserialize_owned(inputs)? {
            io::OpCode::Infer(req) => self.handle_infer(req).await?.to_bytes().map_err(Into::into),
            io::OpCode::Genesis(req) => self
                .handle_genesis(req)
                .await?
                .to_bytes()
                .map_err(Into::into),
            io::OpCode::GetAccountPrimary(req) => self
                .handle_get_account_primary(req)
                .await?
                .to_bytes()
                .map_err(Into::into),
            io::OpCode::SetAccountPrimary(req) => self
                .handle_set_account_primary(req)
                .await?
                .to_bytes()
                .map_err(Into::into),
            io::OpCode::GetAddress(req) => self
                .handle_get_address(req)
                .await?
                .to_bytes()
                .map_err(Into::into),
            io::OpCode::SetAddress(req) => self
                .handle_set_address(req)
                .await?
                .to_bytes()
                .map_err(Into::into),
            io::OpCode::CallRaw(req) => self
                .handle_call_raw(memory, req)
                .await?
                .to_bytes()
                .map_err(Into::into),
            io::OpCode::Release(req) => self
                .handle_release(req)
                .await?
                .to_bytes()
                .map_err(Into::into),
        }
    }
}

#[async_trait]
impl Resource for IpiisHandler {
    async fn release(&mut self) -> Result<()> {
        self.map.release().await?;
        Ok(())
    }
}

impl IpiisHandler {
    async unsafe fn handle_infer(
        &mut self,
        io::request::Infer {}: io::request::Infer,
    ) -> Result<io::response::Infer> {
        let instance = ::ipiis_api::client::IpiisClient::try_infer().await?;
        let account = *instance.account_ref();
        let id = self.map.put(instance);

        Ok(io::response::Infer::new(id, account))
    }

    async unsafe fn handle_genesis(
        &mut self,
        req: io::request::Genesis,
    ) -> Result<io::response::Genesis> {
        let instance = ::ipiis_api::client::IpiisClient::genesis(req.args).await?;
        let account = *instance.account_ref();
        let id = self.map.put(instance);

        Ok(io::response::Infer::new(id, account))
    }

    async unsafe fn handle_get_account_primary(
        &mut self,
        req: io::request::GetAccountPrimary,
    ) -> Result<io::response::GetAccountPrimary> {
        self.map
            .get(&req.id)?
            .get_account_primary(req.kind.as_ref())
            .await
    }

    async unsafe fn handle_set_account_primary(
        &mut self,
        req: io::request::SetAccountPrimary,
    ) -> Result<io::response::SetAccountPrimary> {
        self.map
            .get(&req.id)?
            .set_account_primary(req.kind.as_ref(), &req.account)
            .await
    }

    async unsafe fn handle_get_address(
        &mut self,
        req: io::request::GetAddress,
    ) -> Result<io::response::GetAddress> {
        self.map
            .get(&req.id)?
            .get_address(req.kind.as_ref(), &req.target)
            .await
    }

    async unsafe fn handle_set_address(
        &mut self,
        req: io::request::SetAddress,
    ) -> Result<io::response::SetAddress> {
        self.map
            .get(&req.id)?
            .set_address(req.kind.as_ref(), &req.target, &req.address)
            .await
    }

    async unsafe fn handle_call_raw(
        &mut self,
        memory: &mut IpwisMemory,
        req: io::request::CallRaw,
    ) -> Result<io::response::CallRaw> {
        use core::any::Any;

        let (writer, reader) = self
            .map
            .get(&req.id)?
            .call_raw(req.kind.as_ref(), &req.target)
            .await?;

        // load stream handler
        let stream = memory.get_interrupt_handler(StreamModule.id()).await?;
        let mut stream = stream.lock().await;
        let stream: &mut StreamHandler = (&mut *stream as &mut dyn Any).downcast_mut().unwrap();

        Ok(io::response::CallRaw {
            writer: stream.new_writer(writer)?,
            reader: stream.new_reader(reader)?,
        })
    }

    async unsafe fn handle_release(
        &mut self,
        req: io::request::Release,
    ) -> Result<io::response::Release> {
        self.map.release_one(&req.id).await
    }
}
