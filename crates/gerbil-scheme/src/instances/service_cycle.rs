//! One owner-native transfer, independently rooted projection and bounded refill.
use super::{InstanceError, NativeSession, RetainedValue, image::Image, retained::OwnerRetained};
#[derive(Clone, Copy)]
pub(super) struct ReadyResult {
    pub(super) batch: RetainedValue,
    pub(super) index: usize,
    pub(super) failed: bool,
}
impl ReadyResult {
    pub(super) fn project<'vm>(
        self,
        session: &NativeSession<'vm>,
        taken: &std::cell::Cell<bool>,
    ) -> Result<super::NativeRoot<'vm>, InstanceError> {
        let value = session.take_result_slot(self.batch, self.index)?;
        taken.set(true);
        if self.failed {
            // Match rooted-call diagnostics independently of the application
            // reply quota (4096 Scheme characters, at most 16384 UTF-8 bytes).
            let diagnostic = NativeSession {
                image: session.image,
                maximum_input: 0,
                maximum_output: 16384,
                transferred: std::cell::Cell::new(0),
            };
            Err(InstanceError::Scheme(diagnostic.copy_string(&value)?))
        } else {
            Ok(value)
        }
    }
}
pub(super) struct Batch<'a> {
    root: OwnerRetained<'a>,
    statuses: Vec<i64>,
}
impl Batch<'_> {
    pub(super) fn validate(&self, count: usize) -> Result<(), InstanceError> {
        if self.statuses.len() == count {
            Ok(())
        } else {
            Err(InstanceError::InvalidConfig("native batch result count"))
        }
    }
    pub(super) fn slot(&self, index: usize) -> ReadyResult {
        ReadyResult {
            batch: self.root.value(),
            index,
            failed: self.statuses[index] == 1,
        }
    }
}
pub(super) fn harvest(
    image: &Image,
    epoch: Option<i64>,
) -> Result<(Vec<i64>, Option<Batch<'_>>), InstanceError> {
    let mut result = None;
    super::native_dispatch::run_native(
        image,
        1024,
        (65536 + 5) * 8,
        |session| {
            let header = epoch.map_or_else(|| vec![16], |epoch| vec![16, epoch]);
            let packet = super::service::command(session, &header, None)?;
            let receipt = session.vector_element(&packet, 0)?;
            let receipt = session.copy_numeric::<i64>(&receipt)?;
            let statuses = session.vector_element(&packet, 1)?;
            let statuses = session.copy_numeric::<i64>(&statuses)?;
            if statuses.len() > 65536 || statuses.iter().any(|s| !matches!(s, 0 | 1)) {
                return Err(InstanceError::InvalidConfig("native batch result status"));
            }
            let root = if statuses.is_empty() {
                None
            } else {
                let slots = session.vector_element(&packet, 2)?;
                Some(session.retain_service(slots)?)
            };
            Ok((receipt, statuses, root))
        },
        |value| {
            result = Some(value.0);
            true
        },
    );
    let (receipt, statuses, root) = result.unwrap()?;
    Ok((
        receipt,
        root.map(|value| Batch {
            root: OwnerRetained::new(image, value),
            statuses,
        }),
    ))
}

/// Copy ready identities/statuses and retain only the result batch, in one call.
pub(super) fn harvest_exchange(
    image: &Image,
    maximum_jobs: usize,
) -> Result<(Vec<i64>, Option<Batch<'_>>), InstanceError> {
    let maximum_jobs = maximum_jobs.min(65536);
    let mut metadata = vec![0_i64; 2 * maximum_jobs + 5];
    let mut result = None;
    super::native_dispatch::run_native(
        image,
        0,
        (65536 + 5) * 8,
        |session| {
            let api = image
                .native
                .exchange
                .ok_or(InstanceError::InvalidConfig("native exchange unavailable"))?;
            let capability = session
                .exchange_id()
                .ok_or(InstanceError::InvalidConfig("native exchange disabled"))?;
            let mut root = 0;
            // SAFETY: owner capability, initialized bounded metadata and one root slot
            // remain live during synchronous copying; no borrowed pointer survives.
            let count = unsafe {
                (api.harvest)(
                    capability,
                    metadata.as_mut_ptr(),
                    metadata.len() as u64,
                    &raw mut root,
                )
            };
            let batch = if root > 0 {
                Some(session.own(root)?)
            } else {
                None
            };
            if count < 0 {
                return Err(session.exchange_failure());
            }
            let r = usize::try_from(metadata[0])
                .map_err(|_| InstanceError::InvalidConfig("exchange receipt length"))?;
            let n = usize::try_from(metadata[1])
                .map_err(|_| InstanceError::InvalidConfig("exchange status length"))?;
            if n > maximum_jobs
                || i64::try_from(n).ok() != Some(count)
                || r > maximum_jobs + 3
                || 2 + r + n > metadata.len()
                || (n > 0) != batch.is_some()
            {
                return Err(InstanceError::InvalidConfig("exchange metadata bounds"));
            }
            let statuses = metadata[2 + r..2 + r + n].to_vec();
            if statuses.iter().any(|s| !matches!(s, 0 | 1)) {
                return Err(InstanceError::InvalidConfig("native batch result status"));
            }
            let root = batch.map(|root| session.retain_service(root)).transpose()?;
            Ok((metadata[2..2 + r].to_vec(), statuses, root))
        },
        |value| {
            result = Some(value.0);
            true
        },
    );
    let (receipt, statuses, root) = result.unwrap()?;
    Ok((
        receipt,
        root.map(|value| Batch {
            root: OwnerRetained::new(image, value),
            statuses,
        }),
    ))
}
