use bytecheck::CheckBytes;
#[cfg(target_os = "wasi")]
use ipiis_common::Ipiis;
use ipis::core::{
    account::{AccountRef, GuaranteeSigned, GuarantorSigned},
    metadata::Metadata,
    signed::IsSigned,
    value::hash::Hash,
};
#[cfg(target_os = "wasi")]
use ipis::{
    async_trait::async_trait,
    core::{
        account::Account,
        anyhow::{bail, Result},
        data::Data,
        signature::SignatureSerializer,
    },
    env::Infer,
    log::warn,
};
use ipwis_modules_core_common::resource_store::ResourceId;
pub use ipwis_modules_stream_common::{ExternReader, ExternWriter};
#[cfg(target_os = "wasi")]
use rkyv::{de::deserializers::SharedDeserializeMap, validation::validators::DefaultValidator};
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
#[allow(dead_code)]
pub struct IpiisClient {
    id: ResourceId,
    account: AccountRef,
}

impl IsSigned for IpiisClient {}

#[cfg(not(target_os = "wasi"))]
impl IpiisClient {
    pub fn new(id: ResourceId, account: AccountRef) -> Self {
        Self { id, account }
    }
}

#[cfg(target_os = "wasi")]
#[async_trait]
impl<'a> Infer<'a> for IpiisClient {
    type GenesisArgs = Option<AccountRef>;
    type GenesisResult = Self;

    async fn try_infer() -> Result<Self>
    where
        Self: Sized,
    {
        unsafe { io::request::Infer {}.syscall() }
    }

    async fn genesis(
        args: <Self as Infer<'a>>::GenesisArgs,
    ) -> Result<<Self as Infer<'a>>::GenesisResult> {
        unsafe { io::request::Genesis { args }.syscall() }
    }
}

#[cfg(target_os = "wasi")]
#[async_trait]
impl Ipiis for IpiisClient {
    type Address = ExternAddress;
    type Reader = ExternReader;
    type Writer = ExternWriter;

    unsafe fn account_me(&self) -> Result<&Account> {
        bail!("Direct accessing to Account is not supported in IPWIS.")
    }

    fn account_ref(&self) -> &AccountRef {
        &self.account
    }

    async fn get_account_primary(&self, kind: Option<&Hash>) -> Result<AccountRef> {
        unsafe {
            io::request::GetAccountPrimary {
                id: self.id,
                kind: kind.cloned(),
            }
            .syscall()
        }
    }

    async fn set_account_primary(&self, kind: Option<&Hash>, account: &AccountRef) -> Result<()> {
        unsafe {
            io::request::SetAccountPrimary {
                id: self.id,
                kind: kind.cloned(),
                account: *account,
            }
            .syscall()
        }
    }

    async fn get_address(
        &self,
        kind: Option<&Hash>,
        target: &AccountRef,
    ) -> Result<<Self as Ipiis>::Address> {
        unsafe {
            io::request::GetAddress {
                id: self.id,
                kind: kind.cloned(),
                target: *target,
            }
            .syscall()
        }
    }

    async fn set_address(
        &self,
        kind: Option<&Hash>,
        target: &AccountRef,
        address: &<Self as Ipiis>::Address,
    ) -> Result<()> {
        unsafe {
            io::request::SetAddress {
                id: self.id,
                kind: kind.cloned(),
                target: *target,
                address: address.clone(),
            }
            .syscall()
        }
    }

    fn sign<'a, T>(&self, target: AccountRef, msg: &'a T) -> Result<Data<GuaranteeSigned, &'a T>>
    where
        T: Archive + Serialize<SignatureSerializer> + IsSigned,
        <T as Archive>::Archived: ::core::fmt::Debug + PartialEq,
    {
        Ok(Data {
            metadata: unsafe {
                io::request::SignAsGuarantee {
                    id: self.id,
                    metadata: Metadata::builder().build_unsigned(target, msg)?,
                }
                .syscall()
            }?,
            data: msg,
        })
    }

    fn sign_owned<T>(&self, target: AccountRef, msg: T) -> Result<Data<GuaranteeSigned, T>>
    where
        T: Archive + Serialize<SignatureSerializer> + IsSigned,
        <T as Archive>::Archived: ::core::fmt::Debug + PartialEq,
    {
        Ok(Data {
            metadata: self.sign(target, &msg)?.metadata,
            data: msg,
        })
    }

    fn sign_as_guarantor<T>(
        &self,
        msg: Data<GuaranteeSigned, T>,
    ) -> Result<Data<GuarantorSigned, T>>
    where
        T: IsSigned,
    {
        Ok(Data {
            metadata: unsafe {
                io::request::SignAsGuarantor {
                    id: self.id,
                    metadata: msg.metadata,
                }
                .syscall()
            }?,
            data: msg.data,
        })
    }

    fn protocol(&self) -> Result<String> {
        unsafe { io::request::Protocol { id: self.id }.syscall() }
    }

    async fn call_raw(
        &self,
        kind: Option<&Hash>,
        target: &AccountRef,
    ) -> Result<(<Self as Ipiis>::Writer, <Self as Ipiis>::Reader)> {
        let io::response::CallRaw { writer, reader } = unsafe {
            io::request::CallRaw {
                id: self.id,
                kind: kind.cloned(),
                target: *target,
            }
            .syscall()
        }?;

        Ok((writer, reader))
    }
}

#[cfg(target_os = "wasi")]
impl Drop for IpiisClient {
    fn drop(&mut self) {
        if let Err(error) = unsafe { io::request::Release { id: self.id }.syscall() } {
            warn!("failed to release the IpiisClient: {:x}: {error}", self.id);
        }
    }
}

pub type ExternAddress = ::std::net::SocketAddr;

pub mod io {
    use ipwis_modules_task_common_wasi::interrupt_id::InterruptId;

    use super::*;

    #[derive(Archive, Serialize, Deserialize)]
    #[archive_attr(derive(CheckBytes))]
    pub enum OpCode {
        Infer(self::request::Infer),
        Genesis(self::request::Genesis),
        GetAccountPrimary(self::request::GetAccountPrimary),
        SetAccountPrimary(self::request::SetAccountPrimary),
        GetAddress(self::request::GetAddress),
        SetAddress(self::request::SetAddress),
        SignAsGuarantee(Box<self::request::SignAsGuarantee>),
        SignAsGuarantor(Box<self::request::SignAsGuarantor>),
        Protocol(self::request::Protocol),
        CallRaw(self::request::CallRaw),
        Release(self::request::Release),
    }

    impl IsSigned for OpCode {}

    impl OpCode {
        pub const ID: InterruptId = InterruptId("ipwis_modules_ipiis");

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
        pub struct Infer {}

        impl IsSigned for Infer {}

        #[cfg(target_os = "wasi")]
        impl Infer {
            pub(crate) unsafe fn syscall(self) -> Result<super::response::Infer> {
                super::OpCode::Infer(self).syscall()
            }
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct Genesis {
            pub args: Option<AccountRef>,
        }

        impl IsSigned for Genesis {}

        #[cfg(target_os = "wasi")]
        impl Genesis {
            pub(crate) unsafe fn syscall(self) -> Result<super::response::Genesis> {
                super::OpCode::Genesis(self).syscall()
            }
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct GetAccountPrimary {
            pub id: ResourceId,
            pub kind: Option<Hash>,
        }

        impl IsSigned for GetAccountPrimary {}

        #[cfg(target_os = "wasi")]
        impl GetAccountPrimary {
            pub(crate) unsafe fn syscall(self) -> Result<super::response::GetAccountPrimary> {
                super::OpCode::GetAccountPrimary(self).syscall()
            }
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct SetAccountPrimary {
            pub id: ResourceId,
            pub kind: Option<Hash>,
            pub account: AccountRef,
        }

        impl IsSigned for SetAccountPrimary {}

        #[cfg(target_os = "wasi")]
        impl SetAccountPrimary {
            pub(crate) unsafe fn syscall(self) -> Result<super::response::SetAccountPrimary> {
                super::OpCode::SetAccountPrimary(self).syscall()
            }
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct GetAddress {
            pub id: ResourceId,
            pub kind: Option<Hash>,
            pub target: AccountRef,
        }

        impl IsSigned for GetAddress {}

        #[cfg(target_os = "wasi")]
        impl GetAddress {
            pub(crate) unsafe fn syscall(self) -> Result<super::response::GetAddress> {
                super::OpCode::GetAddress(self).syscall()
            }
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct SetAddress {
            pub id: ResourceId,
            pub kind: Option<Hash>,
            pub target: AccountRef,
            pub address: ExternAddress,
        }

        impl IsSigned for SetAddress {}

        #[cfg(target_os = "wasi")]
        impl SetAddress {
            pub(crate) unsafe fn syscall(self) -> Result<super::response::SetAddress> {
                super::OpCode::SetAddress(self).syscall()
            }
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct SignAsGuarantee {
            pub id: ResourceId,
            pub metadata: Metadata,
        }

        impl IsSigned for SignAsGuarantee {}

        #[cfg(target_os = "wasi")]
        impl SignAsGuarantee {
            pub(crate) unsafe fn syscall(self) -> Result<super::response::SignAsGuarantee> {
                super::OpCode::SignAsGuarantee(Box::new(self)).syscall()
            }
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct SignAsGuarantor {
            pub id: ResourceId,
            pub metadata: GuaranteeSigned,
        }

        impl IsSigned for SignAsGuarantor {}

        #[cfg(target_os = "wasi")]
        impl SignAsGuarantor {
            pub(crate) unsafe fn syscall(self) -> Result<super::response::SignAsGuarantor> {
                super::OpCode::SignAsGuarantor(Box::new(self)).syscall()
            }
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct Protocol {
            pub id: ResourceId,
        }

        impl IsSigned for Protocol {}

        #[cfg(target_os = "wasi")]
        impl Protocol {
            pub(crate) unsafe fn syscall(self) -> Result<super::response::Protocol> {
                super::OpCode::Protocol(self).syscall()
            }
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct CallRaw {
            pub id: ResourceId,
            pub kind: Option<Hash>,
            pub target: AccountRef,
        }

        impl IsSigned for CallRaw {}

        #[cfg(target_os = "wasi")]
        impl CallRaw {
            pub(crate) unsafe fn syscall(self) -> Result<super::response::CallRaw> {
                super::OpCode::CallRaw(self).syscall()
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

        pub type Infer = IpiisClient;

        pub type Genesis = IpiisClient;

        pub type GetAccountPrimary = AccountRef;

        pub type SetAccountPrimary = ();

        pub type GetAddress = ExternAddress;

        pub type SetAddress = ();

        pub type SignAsGuarantee = GuaranteeSigned;

        pub type SignAsGuarantor = GuarantorSigned;

        pub type Protocol = String;

        #[derive(Archive, Serialize, Deserialize)]
        #[archive_attr(derive(CheckBytes))]
        pub struct CallRaw {
            pub writer: ExternWriter,
            pub reader: ExternReader,
        }

        impl IsSigned for CallRaw {}

        pub type Release = ();
    }
}
