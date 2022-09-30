use ipiis_common::{define_io, external_call, Ipiis, ServerResult};
use ipis::{
    async_trait::async_trait,
    core::{
        account::{GuaranteeSigned, GuarantorSigned},
        anyhow::{bail, Result},
        data::Data,
    },
    object::data::ObjectData,
    tokio,
};

pub use ipwis_modules_core_common::resource_store::ResourceId;
pub use ipwis_modules_task_common::{task::Task, task_poll::TaskPoll};
pub use ipwis_modules_task_common_wasi::program::Program;

#[async_trait]
pub trait Ipwis {
    async fn protocol(&self) -> Result<String>;

    async fn task_spawn(
        &self,
        task: Data<GuaranteeSigned, Task>,
    ) -> Result<Data<GuaranteeSigned, ResourceId>>;

    async fn task_poll(
        &self,
        id: Data<GuarantorSigned, ResourceId>,
    ) -> Result<Data<GuaranteeSigned, TaskPoll>>;

    async fn task_wait(&self, id: Data<GuarantorSigned, ResourceId>) -> Result<Box<ObjectData>> {
        loop {
            match self.task_poll(id).await?.data {
                TaskPoll::Pending => tokio::task::yield_now().await,
                TaskPoll::Ready(outputs) => break Ok(outputs),
                TaskPoll::Trap(errors) => bail!("{}", errors.msg),
            }
        }
    }
}

#[async_trait]
impl<IpiisClient> Ipwis for IpiisClient
where
    IpiisClient: Ipiis + Send + Sync,
{
    async fn protocol(&self) -> Result<String> {
        // next target
        let target = self.get_account_primary(KIND.as_ref()).await?;

        // external call
        let (protocol,) = external_call!(
            client: self,
            target: KIND.as_ref() => &target,
            request: crate::io => Protocol,
            sign: self.sign_owned(target, ())?,
            inputs: { },
            outputs: { protocol, },
        );

        // unpack response
        Ok(protocol)
    }

    async fn task_spawn(
        &self,
        task: Data<GuaranteeSigned, Task>,
    ) -> Result<Data<GuaranteeSigned, ResourceId>> {
        // next target
        let target = task.metadata.guarantor;

        // external call
        let (id,) = external_call!(
            client: self,
            target: KIND.as_ref() => &target,
            request: crate::io => Spawn,
            sign: task,
            inputs: { },
            outputs: { id, },
        );

        // unpack response
        Ok(id)
    }

    async fn task_poll(
        &self,
        id: Data<GuarantorSigned, ResourceId>,
    ) -> Result<Data<GuaranteeSigned, TaskPoll>> {
        // next target
        let target = self.get_account_primary(KIND.as_ref()).await?;

        // external call
        let (poll,) = external_call!(
            client: self,
            target: KIND.as_ref() => &target,
            request: crate::io => Poll,
            sign: self.sign_owned(target, ())?,
            inputs: {
                id: id,
            },
            outputs: { poll, },
        );

        // unpack response
        Ok(poll)
    }
}

define_io! {
    Protocol {
        inputs: { },
        input_sign: Data<GuaranteeSigned, ()>,
        outputs: {
            protocol: String,
        },
        output_sign: Data<GuarantorSigned, ()>,
        generics: { },
    },
    Spawn {
        inputs: { },
        input_sign: Data<GuaranteeSigned, Task>,
        outputs: {
            id: Data<GuaranteeSigned, ResourceId>,
        },
        output_sign: Data<GuarantorSigned, Task>,
        generics: { },
    },
    Poll {
        inputs: {
            id: Data<GuarantorSigned, ResourceId>,
        },
        input_sign: Data<GuaranteeSigned, ()>,
        outputs: {
            poll: Data<GuaranteeSigned, TaskPoll>,
        },
        output_sign: Data<GuarantorSigned, ()>,
        generics: { },
    },
}

::ipis::lazy_static::lazy_static! {
    pub static ref KIND: Option<::ipis::core::value::hash::Hash> = Some(
        ::ipis::core::value::hash::Hash::with_str("__ipis__ipwis__"),
    );
}
