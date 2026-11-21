//! Owner-local access to the existing Scheme numeric tower; no f64 coercion.
use super::{InstanceError, NativeRoot, NativeSession};
use crate::NumberKind;
fn tag(kind: NumberKind) -> i32 {
    match kind {
        NumberKind::Rational => 0,
        NumberKind::Complex => 1,
    }
}
impl<'vm> NativeSession<'vm> {
    /// Construct native rational or rectangular complex values from rooted pairs.
    /// Scheme performs exact arithmetic and canonicalization (including real collapse).
    /// # Errors
    /// Foreign roots, input bounds, unsupported ABI or invalid numeric components.
    pub fn number_batch(
        &self,
        kind: NumberKind,
        pairs: &[(&NativeRoot<'_>, &NativeRoot<'_>)],
    ) -> Result<NativeRoot<'vm>, InstanceError> {
        self.input(std::mem::size_of_val(pairs))?;
        let call = self
            .objects()?
            .number_create
            .ok_or(InstanceError::InvalidConfig(
                "native numeric tower unavailable",
            ))?;
        let mut roots = Vec::with_capacity(pairs.len() * 2);
        for (left, right) in pairs {
            self.local(left)?;
            self.local(right)?;
            roots.extend([left.root, right.root]);
        }
        // SAFETY: complete live owner-local component pairs; Scheme retains no Rust span.
        self.own(unsafe { call(roots.as_ptr(), pairs.len() as u64, tag(kind)) })
    }
    /// Snapshot numerator/denominator or real/imaginary components as native roots.
    /// Child identities survive dropping the parent and subsequent GC. A numeric
    /// vector is flattened into alternating component pairs in the same order.
    /// # Errors
    /// Foreign roots, wrong numeric family, output bounds or malformed snapshot.
    pub fn number_components(
        &self,
        value: &NativeRoot<'_>,
        kind: NumberKind,
    ) -> Result<Vec<NativeRoot<'vm>>, InstanceError> {
        self.local(value)?;
        let api = self.objects()?;
        let items = if self.kind(value)? == crate::ObjectKind::Vector {
            // SAFETY: the owner-local vector remains rooted; this reads no elements.
            usize::try_from(unsafe { (api.length)(value.root, 0) })
                .map_err(|_| InstanceError::InvalidConfig("numeric vector length"))?
        } else {
            1
        };
        let expected = items
            .checked_mul(2)
            .filter(|count| *count <= self.maximum_output / 8)
            .ok_or(InstanceError::InvalidConfig("numeric component bound"))?;
        let mut count = 0;
        // SAFETY: input stays rooted; the native ABI returns one rooted shallow snapshot.
        let snapshot =
            self.own(unsafe { (api.composite)(value.root, -1, tag(kind), &raw mut count) })?;
        let count = usize::try_from(count)
            .map_err(|_| InstanceError::InvalidConfig("numeric component count"))?;
        if count != expected {
            return Err(InstanceError::InvalidConfig("numeric component bound"));
        }
        (0..count)
            .map(|index| self.vector_element(&snapshot, index))
            .collect()
    }
}
