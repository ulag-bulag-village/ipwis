use std::sync::Arc;

use ipiis_api::{
    client::IpiisClient,
    common::{handle_external_call, Ipiis, ServerResult},
    server::IpiisServer,
};
use ipis::{async_trait::async_trait, core::anyhow::Result, env::Infer};
use ipwis_common::Ipwis;

use crate::client::IpwisClientInner;

pub struct IpwisServer {
    client: Arc<IpwisClientInner<IpiisServer>>,
}

impl ::core::ops::Deref for IpwisServer {
    type Target = IpwisClientInner<IpiisServer>;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

#[async_trait]
impl<'a> Infer<'a> for IpwisServer {
    type GenesisArgs = <IpiisServer as Infer<'a>>::GenesisArgs;
    type GenesisResult = Self;

    async fn try_infer() -> Result<Self> {
        Ok(Self {
            client: IpwisClientInner::<IpiisServer>::try_infer().await?.into(),
        })
    }

    async fn genesis(
        args: <Self as Infer<'a>>::GenesisArgs,
    ) -> Result<<Self as Infer<'a>>::GenesisResult> {
        Ok(Self {
            client: IpwisClientInner::<IpiisServer>::genesis(args).await?.into(),
        })
    }
}

handle_external_call!(
    server: IpwisServer => IpwisClientInner<IpiisServer>,
    name: run,
    request: ::ipwis_common::io => {
        Protocol => handle_protocol,
        Spawn => handle_spawn,
        Poll => handle_poll,
    },
);

impl IpwisServer {
    async fn handle_protocol(
        client: &IpwisClientInner<IpiisServer>,
        req: ::ipwis_common::io::request::Protocol<'static>,
    ) -> Result<::ipwis_common::io::response::Protocol<'static>> {
        // unpack sign
        let sign_as_guarantee = req.__sign.into_owned().await?;

        // handle data
        let protocol = client.protocol().await?;

        // sign data
        let server: &IpiisServer = client.as_ref();
        let sign = server.sign_as_guarantor(sign_as_guarantee)?;

        // pack data
        Ok(::ipwis_common::io::response::Protocol {
            __lifetime: Default::default(),
            __sign: ::ipis::stream::DynStream::Owned(sign),
            protocol: ::ipis::stream::DynStream::Owned(protocol),
        })
    }

    async fn handle_spawn(
        client: &IpwisClientInner<IpiisServer>,
        req: ::ipwis_common::io::request::Spawn<'static>,
    ) -> Result<::ipwis_common::io::response::Spawn<'static>> {
        // unpack sign
        let sign_as_guarantee = req.__sign.into_owned().await?;

        // unpack data
        let ctx = sign_as_guarantee.clone();

        // handle data
        let id = client.task_spawn(ctx).await?;

        // sign data
        let server: &IpiisServer = client.as_ref();
        let sign = server.sign_as_guarantor(sign_as_guarantee)?;

        // pack data
        Ok(::ipwis_common::io::response::Spawn {
            __lifetime: Default::default(),
            __sign: ::ipis::stream::DynStream::Owned(sign),
            id: ::ipis::stream::DynStream::Owned(id),
        })
    }

    async fn handle_poll(
        client: &IpwisClientInner<IpiisServer>,
        req: ::ipwis_common::io::request::Poll<'static>,
    ) -> Result<::ipwis_common::io::response::Poll<'static>> {
        // unpack sign
        let sign_as_guarantee = req.__sign.into_owned().await?;

        // unpack data
        let id = req.id.into_owned().await?;

        // handle data
        let poll = client.task_poll(id).await?;

        // sign data
        let server: &IpiisServer = client.as_ref();
        let sign = server.sign_as_guarantor(sign_as_guarantee)?;

        // pack data
        Ok(::ipwis_common::io::response::Poll {
            __lifetime: Default::default(),
            __sign: ::ipis::stream::DynStream::Owned(sign),
            poll: ::ipis::stream::DynStream::Owned(poll),
        })
    }
}
