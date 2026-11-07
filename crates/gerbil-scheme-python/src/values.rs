// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Owned data shared by C ABI and Python adapters, independent of either runtime.
use num_bigint::BigInt;
use std::collections::BTreeMap;

/// Owned data, not a Scheme heap pointer or authority to enter a VM.
#[derive(Clone, Debug, PartialEq)]
pub enum OwnedValue {
    Null,
    Boolean(bool),
    Integer(BigInt),
    Real(f64),
    Text(String),
    Bytes(Vec<u8>),
    List(Vec<Self>),
    Map(BTreeMap<String, Self>),
}
/// Structural decoding bounds; independent of application fact/request policy.
#[derive(Clone, Copy, Debug)]
pub struct ValueLimits {
    pub depth: usize,
    pub nodes: usize,
}
impl Default for ValueLimits {
    fn default() -> Self {
        Self {
            depth: 128,
            nodes: 1_000_000,
        }
    }
}
