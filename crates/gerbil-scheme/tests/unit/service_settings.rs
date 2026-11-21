//! A strategy declaration cannot substitute for valid native settings metadata.
use super::{SchemeServiceSettings, SchemeServiceWake};
#[test]
fn native_settings_reject_invalid_or_partial_metadata() {
    assert_eq!(
        SchemeServiceSettings::parse(&[0, 0]).unwrap(),
        SchemeServiceSettings {
            cooperative_budget: 0,
            wake: SchemeServiceWake::Watcher
        }
    );
    assert_eq!(
        SchemeServiceSettings::parse(&[256, 1]).unwrap(),
        SchemeServiceSettings {
            cooperative_budget: 256,
            wake: SchemeServiceWake::NativePort
        }
    );
    for invalid in [
        &[][..],
        &[1],
        &[1, 1, 1],
        &[-1, 0],
        &[257, 0],
        &[1, -1],
        &[1, 2],
    ] {
        assert!(SchemeServiceSettings::parse(invalid).is_err());
    }
}
