//! Attribution, not additive phase costs or a replacement throughput gate.
use super::{Root, convert, gerbil_utf8_comparison_snapshot, gerbil_utf8_comparison_stat, median};
use gerbil_scheme_sys::GerbilStatus;
use std::time::Instant;

unsafe extern "C" {
    fn gerbil_utf8_snapshot_overhead() -> f64;
    fn gerbil_utf8_os_snapshot(slot: i32);
    fn gerbil_utf8_os_stat(index: i32) -> f64;
    fn gerbil_utf8_leaf_dispatch(root: i64, output: *mut u8, capacity: u64) -> i64;
}

const PHASES: [&str; 7] = [
    "owned",
    "length",
    "allocation",
    "encoding",
    "validation",
    "entry",
    "dispatch",
];

pub(super) fn run(values: &[Root], texts: &[String], empty: &Root) {
    for jobs in [1_000, 10_000, 100_000] {
        let mut samples: [Vec<[f64; 9]>; 7] = std::array::from_fn(|_| Vec::new());
        for round in 0..20 {
            // Rotation balances phase chronology without silently pooling loads.
            for position in 0..PHASES.len() {
                let phase = (round + position) % PHASES.len();
                let sample = measure(values, texts, empty, jobs, phase);
                samples[phase].push(sample);
                println!(
                    "UTF8-COST jobs={jobs} phase={} sample={} wall_ms={:.3} cpu_ms={:.3} gc_cpu_ms={:.3} gc_wall_ms={:.3} scheme_bytes={:.0} minor_faults={:.0} voluntary_switches={:.0} involuntary_switches={:.0} thread_cpu_ms={:.3} wall_minus_thread_cpu_ms={:.3}",
                    PHASES[phase],
                    round + 1,
                    sample[0] * 1000.0,
                    sample[1] * 1000.0,
                    sample[2] * 1000.0,
                    sample[3] * 1000.0,
                    sample[4],
                    sample[5],
                    sample[6],
                    sample[7],
                    sample[8] * 1000.0,
                    (sample[0] - sample[8]) * 1000.0
                );
            }
        }
        for (phase, records) in samples.iter().enumerate() {
            let medians: [f64; 9] = std::array::from_fn(|index| {
                median(&records.iter().map(|row| row[index]).collect::<Vec<_>>())
            });
            println!(
                "UTF8-COST-SUMMARY jobs={jobs} phase={} samples=20 wall_ms={:.3} cpu_ms={:.3} gc_cpu_ms={:.3} gc_wall_ms={:.3} scheme_bytes={:.0} minor_faults={:.0} voluntary_switches={:.0} involuntary_switches={:.0} thread_cpu_ms={:.3}",
                PHASES[phase],
                medians[0] * 1000.0,
                medians[1] * 1000.0,
                medians[2] * 1000.0,
                medians[3] * 1000.0,
                medians[4],
                medians[5],
                medians[6],
                medians[7],
                medians[8] * 1000.0
            );
        }
    }
}

fn measure(values: &[Root], texts: &[String], empty: &Root, jobs: usize, phase: usize) -> [f64; 9] {
    // One unfilled scratch allocation outside every phase keeps preparation
    // uniform. Only the explicitly isolated encoding phase writes it.
    // The owned end-to-end route still includes every allocation and validation.
    let capacity = texts
        .iter()
        .map(|text| 4 * text.chars().count())
        .max()
        .unwrap();
    let mut scratch = Vec::<u8>::with_capacity(capacity);
    if phase == 6 {
        scratch.resize(capacity, 0xa5);
    }
    // SAFETY: all snapshots belong to this initialized runtime owner.
    unsafe {
        gerbil_utf8_comparison_snapshot(0);
        gerbil_utf8_os_snapshot(0);
    }
    let start = Instant::now();
    for index in 0..jobs {
        operation(
            if phase == 5 {
                empty
            } else {
                &values[index % values.len()]
            },
            &texts[index % texts.len()],
            phase,
            &mut scratch,
        );
    }
    let wall = start.elapsed().as_secs_f64();
    // SAFETY: completed owner-local sample; counters are outside timing.
    unsafe {
        gerbil_utf8_os_snapshot(1);
        gerbil_utf8_comparison_snapshot(1);
    }
    if phase == 6 {
        assert!(
            scratch.iter().all(|byte| *byte == 0xa5),
            "dispatch modified destination"
        );
    }
    let counter = |index| unsafe { gerbil_utf8_comparison_stat(index) };
    let os = |index| unsafe { gerbil_utf8_os_stat(index) };
    let bytes = counter(7) - unsafe { gerbil_utf8_snapshot_overhead() };
    let row = [
        wall,
        counter(0) + counter(1),
        counter(3) + counter(4),
        counter(5),
        bytes,
        os(0),
        os(2),
        os(3),
        os(4),
    ];
    assert!(
        row.iter().all(|value| value.is_finite() && *value >= 0.0),
        "unavailable cost statistic"
    );
    // The OS returns an integral count cast to double. Require exact positive
    // zero bits: no epsilon or tolerance may admit a major page fault.
    assert_eq!(
        os(1).to_bits(),
        0.0_f64.to_bits(),
        "major page fault: attribution sample is not resident"
    );
    row
}

fn operation(value: &Root, text: &str, phase: usize, scratch: &mut Vec<u8>) {
    match phase {
        0 => assert_eq!(std::hint::black_box(convert(value, 5)), text),
        1 => {
            let mut characters = 0;
            // SAFETY: live owner-local root, writable length output.
            assert_eq!(
                unsafe {
                    gerbil_scheme_sys::gerbil_scheme_rust_root_string_length(
                        value.0,
                        &raw mut characters,
                    )
                },
                GerbilStatus::Ok
            );
            assert_eq!(std::hint::black_box(characters), 3415);
        }
        2 => {
            let output = Vec::<u8>::with_capacity(scratch.capacity());
            assert!(output.capacity() >= scratch.capacity());
            std::hint::black_box(output);
        }
        3 => {
            scratch.clear();
            let mut written = 0;
            // SAFETY: live root and stable, exclusive writable Rust capacity.
            assert_eq!(
                unsafe {
                    gerbil_scheme_sys::gerbil_scheme_rust_root_string_encode_into(
                        value.0,
                        scratch.as_mut_ptr(),
                        scratch.capacity(),
                        &raw mut written,
                    )
                },
                GerbilStatus::Ok
            );
            assert!(written <= scratch.capacity());
            // SAFETY: only full successful initialization admits this prefix.
            unsafe { scratch.set_len(written) };
            assert_eq!(std::hint::black_box(scratch.as_slice()), text.as_bytes());
        }
        4 => {
            let valid = simdutf8::basic::from_utf8(std::hint::black_box(text.as_bytes())).unwrap();
            assert_eq!(std::hint::black_box(valid), text);
        }
        5 => {
            // Success-path entry floor: actual checked ABI, null destination,
            // and an empty string needing no encoding leaves. Null-pointer
            // marshalling differs from a non-null destination's wrapper.
            // This is not subtracted from the changing-text encoding phase.
            let mut written = usize::MAX;
            // SAFETY: live empty root; capacity zero permits a null destination.
            assert_eq!(
                unsafe {
                    gerbil_scheme_sys::gerbil_scheme_rust_root_string_encode_into(
                        value.0,
                        std::ptr::null_mut(),
                        0,
                        &raw mut written,
                    )
                },
                GerbilStatus::Ok
            );
            assert_eq!(std::hint::black_box(written), 0);
        }
        6 => {
            // SAFETY: exclusive initialized canary capacity and a live owner
            // root; checked empty leaves must not modify any destination byte.
            assert_eq!(
                unsafe {
                    gerbil_utf8_leaf_dispatch(
                        value.0.0,
                        scratch.as_mut_ptr(),
                        u64::try_from(scratch.capacity()).unwrap(),
                    )
                },
                0
            );
        }
        _ => unreachable!("invalid diagnostic phase"),
    }
}
