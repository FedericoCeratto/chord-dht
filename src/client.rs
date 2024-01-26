use crate::{core::DhtResult, rpc::NodeServiceClient};
use log::info;
use tarpc::tokio_serde::formats::Bincode;

pub async fn setup_client(addr: &str) -> DhtResult<NodeServiceClient> {
    info!("connecting to {}", addr);
    let transport = tarpc::serde_transport::tcp::connect(addr, Bincode::default).await?;
    info!("connected to {}", addr);
    Ok(NodeServiceClient::new(tarpc::client::Config::default(), transport).spawn())
}
