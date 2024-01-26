// SPDX-FileCopyrightText: 2022 DCsunset
//
// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod config;
pub mod data_store;
pub mod error;
pub mod node;
pub mod ring;

pub use config::*;
pub use error::*;
pub use node::*;

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

pub fn calculate_hash(data: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

pub fn construct_node(addr: &str) -> Node {
    Node {
        addr: addr.to_string(),
        id: calculate_hash(addr.as_bytes()),
    }
}
