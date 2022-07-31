#[cfg(target_os = "wasi")]
use core::{
    pin::Pin,
    task::{Context, Poll},
};

use bytecheck::CheckBytes;
#[cfg(target_os = "wasi")]
use ipis::{
    core::anyhow::Error,
    log::warn,
    tokio::{
        self,
        io::{AsyncRead, AsyncWrite, ReadBuf},
    },
};
use ipwis_modules_core_common::resource_store::ResourceId;
use ipwis_modules_task_common_wasi::extern_data::{ExternData, ExternDataRef};
#[cfg(target_os = "wasi")]
use rkyv::{de::deserializers::SharedDeserializeMap, validation::validators::DefaultValidator};
use rkyv::{Archive, Deserialize, Serialize};

#[allow(dead_code)]
pub struct ExternReader {
    id: ResourceId,
    len: ExternDataRef,
}

#[cfg(not(target_os = "wasi"))]
impl ExternReader {
    pub fn new(id: ResourceId, len: ExternDataRef) -> Self {
        Self { id, len }
    }
}

#[cfg(target_os = "wasi")]
impl TryFrom<&'_ [u8]> for ExternReader {
    type Error = Error;

    fn try_from(buf: &'_ [u8]) -> Result<Self, Self::Error> {
        let ptr = buf.as_ptr() as ExternDataRef;
        let len = buf.len() as ExternDataRef;

        Ok(Self {
            id: unsafe {
                io::request::ReaderNew {
                    buf: ExternData { ptr, len },
                }
                .syscall()?
                .id
            },
            len,
        })
    }
}

#[cfg(target_os = "wasi")]
impl AsyncRead for ExternReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<tokio::io::Result<()>> {
        unsafe {
            // fill in buffer
            let opcode = self::io::request::ReaderNext {
                id: self.id,
                buf: {
                    let unfilled_buf = buf.unfilled_mut();
                    let len = unfilled_buf.len();

                    let unfilled_buf = ExternData::from_slice_uninit_mut(unfilled_buf);

                    // use the whole buffer
                    buf.assume_init(len);

                    unfilled_buf
                },
            };
            buf.set_filled(opcode.syscall()?.len as usize);
        };
        Poll::Ready(Ok(()))
    }
}

#[cfg(target_os = "wasi")]
impl Drop for ExternReader {
    fn drop(&mut self) {
        if let Err(error) = unsafe { io::request::ReaderRelease { id: self.id }.syscall() } {
            warn!("failed to release the ExternReader: {:x}: {error}", self.id);
        }
    }
}

#[allow(dead_code)]
pub struct ExternWriter {
    id: ResourceId,
}

#[cfg(not(target_os = "wasi"))]
impl ExternWriter {
    pub fn new(id: ResourceId) -> Self {
        Self { id }
    }
}

#[cfg(target_os = "wasi")]
impl AsyncWrite for ExternWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, tokio::io::Error>> {
        let len = unsafe {
            let opcode = self::io::request::WriterNext {
                id: self.id,
                buf: ExternData::from_slice(buf),
            };
            opcode.syscall()?.len
        };
        Poll::Ready(Ok(len as usize))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), tokio::io::Error>> {
        unsafe {
            let opcode = self::io::request::WriterFlush { id: self.id };
            opcode.syscall()?;
        };
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Result<(), tokio::io::Error>> {
        unsafe {
            let opcode = self::io::request::WriterShutdown { id: self.id };
            opcode.syscall()?;
        };
        Poll::Ready(Ok(()))
    }
}

#[cfg(target_os = "wasi")]
impl Drop for ExternWriter {
    fn drop(&mut self) {
        if let Err(error) = unsafe { io::request::WriterRelease { id: self.id }.syscall() } {
            warn!("failed to release the ExternWriter: {:x}: {error}", self.id);
        }
    }
}

pub mod io {
    use ipwis_modules_task_common_wasi::interrupt_id::InterruptId;

    use super::*;

    #[derive(Archive, Serialize, Deserialize)]
    #[archive_attr(derive(CheckBytes))]
    pub enum OpCode {
        ReaderNew(self::request::ReaderNew),
        ReaderNext(self::request::ReaderNext),
        ReaderRelease(self::request::ReaderRelease),
        WriterNext(self::request::WriterNext),
        WriterFlush(self::request::WriterFlush),
        WriterShutdown(self::request::WriterShutdown),
        WriterRelease(self::request::WriterRelease),
    }

    impl ::ipis::core::signed::IsSigned for OpCode {}

    impl OpCode {
        pub const ID: InterruptId = InterruptId("ipwis_modules_stream");

        #[cfg(target_os = "wasi")]
        unsafe fn syscall<O>(mut self) -> Result<O, tokio::io::Error>
        where
            O: Archive,
            <O as Archive>::Archived:
                for<'a> CheckBytes<DefaultValidator<'a>> + Deserialize<O, SharedDeserializeMap>,
        {
            Self::ID
                .syscall(&mut self)
                .map_err(|e| tokio::io::Error::new(tokio::io::ErrorKind::Other, e))
        }
    }

    pub mod request {
        use super::*;

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct ReaderNew {
            pub buf: ExternData,
        }

        impl ::ipis::core::signed::IsSigned for ReaderNew {}

        #[cfg(target_os = "wasi")]
        impl ReaderNew {
            pub(crate) unsafe fn syscall(
                self,
            ) -> Result<super::response::ReaderNew, tokio::io::Error> {
                super::OpCode::ReaderNew(self).syscall()
            }
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct ReaderNext {
            pub id: ResourceId,
            pub buf: ExternData,
        }

        impl ::ipis::core::signed::IsSigned for ReaderNext {}

        #[cfg(target_os = "wasi")]
        impl ReaderNext {
            pub(crate) unsafe fn syscall(
                self,
            ) -> Result<super::response::ReaderNext, tokio::io::Error> {
                super::OpCode::ReaderNext(self).syscall()
            }
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct ReaderRelease {
            pub id: ResourceId,
        }

        impl ::ipis::core::signed::IsSigned for ReaderRelease {}

        #[cfg(target_os = "wasi")]
        impl ReaderRelease {
            pub(crate) unsafe fn syscall(
                self,
            ) -> Result<super::response::ReaderRelease, tokio::io::Error> {
                super::OpCode::ReaderRelease(self).syscall()
            }
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct WriterNext {
            pub id: ResourceId,
            pub buf: ExternData,
        }

        impl ::ipis::core::signed::IsSigned for WriterNext {}

        #[cfg(target_os = "wasi")]
        impl WriterNext {
            pub(crate) unsafe fn syscall(
                self,
            ) -> Result<super::response::WriterNext, tokio::io::Error> {
                super::OpCode::WriterNext(self).syscall()
            }
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct WriterFlush {
            pub id: ResourceId,
        }

        impl ::ipis::core::signed::IsSigned for WriterFlush {}

        #[cfg(target_os = "wasi")]
        impl WriterFlush {
            pub(crate) unsafe fn syscall(
                self,
            ) -> Result<super::response::WriterFlush, tokio::io::Error> {
                super::OpCode::WriterFlush(self).syscall()
            }
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct WriterShutdown {
            pub id: ResourceId,
        }

        impl ::ipis::core::signed::IsSigned for WriterShutdown {}

        #[cfg(target_os = "wasi")]
        impl WriterShutdown {
            pub(crate) unsafe fn syscall(
                self,
            ) -> Result<super::response::WriterShutdown, tokio::io::Error> {
                super::OpCode::WriterShutdown(self).syscall()
            }
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct WriterRelease {
            pub id: ResourceId,
        }

        impl ::ipis::core::signed::IsSigned for WriterRelease {}

        #[cfg(target_os = "wasi")]
        impl WriterRelease {
            pub(crate) unsafe fn syscall(
                self,
            ) -> Result<super::response::WriterRelease, tokio::io::Error> {
                super::OpCode::WriterRelease(self).syscall()
            }
        }
    }

    pub mod response {
        use super::*;

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct ReaderNew {
            pub id: ResourceId,
        }

        impl ::ipis::core::signed::IsSigned for ReaderNew {}

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct ReaderNext {
            pub len: ExternDataRef,
        }

        impl ::ipis::core::signed::IsSigned for ReaderNext {}

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct ReaderRelease {}

        impl ::ipis::core::signed::IsSigned for ReaderRelease {}

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct WriterNext {
            pub len: ExternDataRef,
        }

        impl ::ipis::core::signed::IsSigned for WriterNext {}

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct WriterFlush {}

        impl ::ipis::core::signed::IsSigned for WriterFlush {}

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct WriterShutdown {}

        impl ::ipis::core::signed::IsSigned for WriterShutdown {}

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct WriterRelease {}

        impl ::ipis::core::signed::IsSigned for WriterRelease {}
    }
}
