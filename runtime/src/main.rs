use ipis::{env::Infer, tokio};
use ipwis_api::server::IpwisServer;

#[tokio::main]
async fn main() {
    IpwisServer::infer().await.run().await
}
