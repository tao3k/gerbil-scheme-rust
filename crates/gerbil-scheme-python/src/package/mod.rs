// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
//! Complete resource inventories and checked generated-root publication.
mod inventory;
#[cfg(feature = "parallel-verification")]
mod parallel;
mod publication;
pub use inventory::{PackageManifest, PackageResource, VerifiedPackage, sha256_file};
