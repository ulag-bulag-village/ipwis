#![allow(clippy::missing_safety_doc)]
#![allow(incomplete_features)]
#![feature(trait_upcasting)]

use std::io::Cursor;

use ipis::{
    async_trait::async_trait,
    core::{anyhow::Result, signed::IsSigned},
    env::Infer,
    pin::PinnedInner,
    resource::Resource,
    rkyv::AlignedVec,
};
use ipwis_modules_core_common::resource_store::ResourceStore;
use ipwis_modules_stream_api::{StreamHandler, StreamModule};
use ipwis_modules_task_api_wasi::{
    interrupt_handler::InterruptHandler, interrupt_module::InterruptModule, memory::IpwisMemory,
};
use ipwis_modules_task_common_wasi::interrupt_id::InterruptId;
use ipwis_modules_webcam_common::io;

#[derive(Copy, Clone, Debug, Default)]
pub struct WebcamModule;

#[async_trait]
impl InterruptModule for WebcamModule {
    fn id(&self) -> InterruptId {
        io::OpCode::ID
    }

    async fn spawn_handler(&self) -> Result<Box<dyn InterruptHandler>> {
        Ok(Box::new(WebcamHandler {
            map: Default::default(),
        }))
    }
}

pub struct WebcamHandler {
    map: ResourceStore<WebcamInstance>,
}

#[async_trait]
impl InterruptHandler for WebcamHandler {
    async unsafe fn handle_raw(
        &mut self,
        memory: &mut IpwisMemory,
        inputs: &[u8],
    ) -> Result<AlignedVec> {
        match PinnedInner::deserialize_owned(inputs)? {
            io::OpCode::New(req) => self.handle_new(req).await?.to_bytes().map_err(Into::into),
            io::OpCode::CaptureFrame(req) => self
                .handle_capture_frame(memory, req)
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
impl Resource for WebcamHandler {
    async fn release(&mut self) -> Result<()> {
        self.map.release().await?;
        Ok(())
    }
}

impl WebcamHandler {
    async unsafe fn handle_new(
        &mut self,
        io::request::New {}: io::request::New,
    ) -> Result<io::response::New> {
        let instance = WebcamInstance::try_infer().await?;
        let id = self.map.put(instance);

        Ok(io::response::New::new(id))
    }

    async unsafe fn handle_capture_frame(
        &mut self,
        memory: &mut IpwisMemory,
        req: io::request::CaptureFrame,
    ) -> Result<io::response::CaptureFrame> {
        use core::any::Any;

        let image_buffer = self.map.get_mut(&req.id)?.0.frame()?;

        // load stream handler
        let stream = memory.get_interrupt_handler(StreamModule.id()).await?;
        let mut stream = stream.lock().await;
        #[allow(clippy::explicit_auto_deref)]
        let stream: &mut dyn InterruptHandler<IpwisMemory> = &mut **stream;
        let stream: &mut StreamHandler = (stream as &mut dyn Any).downcast_mut().unwrap();

        stream.new_reader(Cursor::new(image_buffer.into_vec()))
    }

    async unsafe fn handle_release(
        &mut self,
        io::request::Release { id }: io::request::Release,
    ) -> Result<io::response::Release> {
        self.map.release_one(&id).await
    }
}

pub struct WebcamInstance(::nokhwa::Camera);

// ## Safety
// This instance is used in only one thread
unsafe impl Send for WebcamInstance {}
unsafe impl Sync for WebcamInstance {}

#[async_trait]
impl<'a> Infer<'a> for WebcamInstance {
    type GenesisArgs = (usize, Option<::nokhwa::CameraFormat>);
    type GenesisResult = Self;

    async fn try_infer() -> Result<Self>
    where
        Self: Sized,
    {
        // TODO: parse from external values
        let index = 0;
        let format = None;
        <Self as Infer<'a>>::genesis((index, format)).await
    }

    async fn genesis(
        (index, format): <Self as Infer<'a>>::GenesisArgs,
    ) -> Result<<Self as Infer<'a>>::GenesisResult> {
        ::nokhwa::Camera::new(index, format)
            .map(Self)
            .map_err(Into::into)
    }
}

#[async_trait]
impl Resource for WebcamInstance {
    async fn release(&mut self) -> Result<()> {
        self.0.stop_stream().map_err(Into::into)
    }
}
