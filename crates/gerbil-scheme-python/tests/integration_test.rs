// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
#[path = "integration/backend.rs"]
mod backend;
#[path = "integration/package.rs"]
mod package;
#[cfg(feature = "python")]
#[path = "integration/owned_value.rs"]
mod owned_value;
#[path = "integration/owned_state.rs"]
mod owned_state;
#[cfg(feature = "c-abi")]
#[path = "integration/c_abi.rs"]
mod c_abi;
