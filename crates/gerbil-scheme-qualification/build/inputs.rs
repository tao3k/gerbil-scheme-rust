//! Original producer and SDK inputs are tracked; this build's outputs are not.

use std::path::Path;

pub(super) fn external_input(source: &Path, out: &Path) -> bool {
    !source.starts_with(out)
}
