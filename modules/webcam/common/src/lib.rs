use bytecheck::CheckBytes;
#[cfg(target_os = "wasi")]
use ipis::core::log::warn;
use ipis::{
    async_trait::async_trait,
    core::{anyhow::Result, signed::IsSigned},
};
use ipwis_modules_core_common::resource_store::ResourceId;
use ipwis_modules_stream_common::ExternReader;
#[cfg(target_os = "wasi")]
use rkyv::{de::deserializers::SharedDeserializeMap, validation::validators::DefaultValidator};
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
#[allow(dead_code)]
pub struct WebcamClient {
    id: ResourceId,
}

impl IsSigned for WebcamClient {}

#[cfg(not(target_os = "wasi"))]
impl WebcamClient {
    pub fn new(id: ResourceId) -> Self {
        Self { id }
    }
}

#[cfg(target_os = "wasi")]
impl WebcamClient {
    pub async fn new() -> Result<Self> {
        unsafe { io::request::New {}.syscall() }
    }
}

#[async_trait]
pub trait Webcam {
    async fn capture_frame(&self) -> Result<ExternReader>;
}

#[cfg(target_os = "wasi")]
#[async_trait]
impl Webcam for WebcamClient {
    async fn capture_frame(&self) -> Result<ExternReader> {
        unsafe { io::request::CaptureFrame { id: self.id }.syscall() }
    }
}

#[cfg(target_os = "wasi")]
impl Drop for WebcamClient {
    fn drop(&mut self) {
        if let Err(error) = unsafe { io::request::Release { id: self.id }.syscall() } {
            warn!("failed to release the WebcamClient: {:x}: {error}", self.id);
        }
    }
}

pub mod io {
    use ipwis_modules_task_common_wasi::interrupt_id::InterruptId;

    use super::*;

    #[derive(Archive, Serialize, Deserialize)]
    #[archive_attr(derive(CheckBytes))]
    pub enum OpCode {
        New(self::request::New),
        CaptureFrame(self::request::CaptureFrame),
        Release(self::request::Release),
    }

    impl IsSigned for OpCode {}

    impl OpCode {
        pub const ID: InterruptId = InterruptId("ipwis_modules_webcam");

        #[cfg(target_os = "wasi")]
        unsafe fn syscall<O>(mut self) -> Result<O>
        where
            O: Archive,
            <O as Archive>::Archived:
                for<'a> CheckBytes<DefaultValidator<'a>> + Deserialize<O, SharedDeserializeMap>,
        {
            Self::ID.syscall(&mut self)
        }
    }

    pub mod request {
        use super::*;

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct New {}

        impl IsSigned for New {}

        #[cfg(target_os = "wasi")]
        impl New {
            pub(crate) unsafe fn syscall(self) -> Result<super::response::New> {
                super::OpCode::New(self).syscall()
            }
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct CaptureFrame {
            pub id: ResourceId,
        }

        impl IsSigned for CaptureFrame {}

        #[cfg(target_os = "wasi")]
        impl CaptureFrame {
            pub(crate) unsafe fn syscall(self) -> Result<super::response::CaptureFrame> {
                super::OpCode::CaptureFrame(self).syscall()
            }
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct Release {
            pub id: ResourceId,
        }

        impl IsSigned for Release {}

        #[cfg(target_os = "wasi")]
        impl Release {
            pub(crate) unsafe fn syscall(self) -> Result<super::response::Release> {
                super::OpCode::Release(self).syscall()
            }
        }
    }

    pub mod response {
        use super::*;

        pub type New = WebcamClient;

        pub type CaptureFrame = ExternReader;

        pub type Release = ();
    }
}
