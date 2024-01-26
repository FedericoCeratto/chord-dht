// SPDX-FileCopyrightText: 2022 DCsunset
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{core::DhtResult, rpc::NodeServiceClient};
use log::info;
use tarpc::tokio_serde::formats::Bincode;

pub async fn setup_client(addr: &str) -> DhtResult<NodeServiceClient> {
    info!("connecting to {}", addr);
    let transport = tarpc::serde_transport::tcp::connect(addr, Bincode::default).await?;
    info!("connected to {}", addr);
    Ok(NodeServiceClient::new(tarpc::client::Config::default(), transport).spawn())
}
