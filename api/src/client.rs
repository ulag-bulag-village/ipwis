use ipiis_api::common::Ipiis;
use ipis::{
    async_trait::async_trait,
    core::{
        account::{GuaranteeSigned, GuarantorSigned},
        anyhow::{bail, Result},
        data::Data,
    },
    env::Infer,
    futures::TryFutureExt,
    object::data::ObjectData,
};
use ipsis_common::Ipsis;
use ipwis_common::{Ipwis, ResourceId, Task, TaskPoll};
use ipwis_kernel::Kernel;

pub type IpwisClient = IpwisClientInner<::ipiis_api::client::IpiisClient>;

pub struct IpwisClientInner<IpiisClient> {
    pub ipiis: IpiisClient,
    kernel: Kernel,
}

impl<IpiisClient> AsRef<::ipiis_api::client::IpiisClient> for IpwisClientInner<IpiisClient>
where
    IpiisClient: AsRef<::ipiis_api::client::IpiisClient>,
{
    fn as_ref(&self) -> &::ipiis_api::client::IpiisClient {
        self.ipiis.as_ref()
    }
}

impl<IpiisClient> AsRef<::ipiis_api::server::IpiisServer> for IpwisClientInner<IpiisClient>
where
    IpiisClient: AsRef<::ipiis_api::server::IpiisServer>,
{
    fn as_ref(&self) -> &::ipiis_api::server::IpiisServer {
        self.ipiis.as_ref()
    }
}

#[async_trait]
impl<'a, IpiisClient> Infer<'a> for IpwisClientInner<IpiisClient>
where
    Self: Send,
    IpiisClient: Infer<'a, GenesisResult = IpiisClient> + Send,
    <IpiisClient as Infer<'a>>::GenesisArgs: Sized,
{
    type GenesisArgs = <IpiisClient as Infer<'a>>::GenesisArgs;
    type GenesisResult = Self;

    async fn try_infer() -> Result<Self> {
        IpiisClient::try_infer()
            .and_then(Self::with_ipiis_client)
            .await
    }

    async fn genesis(
        args: <Self as Infer<'a>>::GenesisArgs,
    ) -> Result<<Self as Infer<'a>>::GenesisResult> {
        IpiisClient::genesis(args)
            .and_then(Self::with_ipiis_client)
            .await
    }
}

impl<IpiisClient> IpwisClientInner<IpiisClient> {
    pub async fn with_ipiis_client(ipiis: IpiisClient) -> Result<Self> {
        Ok(Self {
            ipiis,
            kernel: Kernel::try_new().await?,
        })
    }
}

#[async_trait]
impl<IpiisClient> Ipwis for IpwisClientInner<IpiisClient>
where
    IpiisClient: Ipiis + Ipsis + Send + Sync,
{
    async fn task_spawn(
        &self,
        task: Data<GuaranteeSigned, Task>,
    ) -> Result<Data<GuaranteeSigned, ResourceId>> {
        let task = self.ipiis.sign_as_guarantor(task)?;
        let guarantee = task.metadata.guarantee.account;

        match &task.program {
            Some(program) => {
                let program: Vec<u8> = self.ipiis.get(program).await?;
                let id = self.kernel.spawn_raw(task, &program).await?;
                self.ipiis.sign_owned(guarantee, id)
            }
            None => bail!("Empty program"),
        }
    }

    async fn task_poll(
        &self,
        id: Data<GuarantorSigned, ResourceId>,
    ) -> Result<Data<GuaranteeSigned, TaskPoll>> {
        let guarantee = id.metadata.guarantee.account;

        self.kernel
            .poll(&id.data)
            .await
            .and_then(|poll| self.ipiis.sign_owned(guarantee, poll))
    }

    async fn task_wait(&self, id: Data<GuarantorSigned, ResourceId>) -> Result<Box<ObjectData>> {
        self.kernel.wait(&id.data).await
    }
}
