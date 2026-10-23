//! One owner-native transfer, independently rooted projection and bounded refill.
use super::{InstanceError, NativeSession, RetainedValue, image::Image, retained::OwnerRetained};
#[derive(Clone, Copy)]
pub(super) struct ReadyResult {
    batch: RetainedValue,
    index: usize,
    failed: bool,
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
