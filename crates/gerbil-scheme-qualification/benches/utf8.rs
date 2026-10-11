//! Same-process ABBA controls; actual owned bridge transfers, never Org parsing.
use std::time::Instant;

use gerbil_scheme::GerbilRuntime;
use gerbil_scheme_sys::{GerbilRootId, GerbilStatus};

#[path = "utf8/buffer.rs"]
mod buffer;

unsafe extern "C" {
    fn gerbil_utf8_comparison_encode(root: i64, mode: i32) -> i64;
    fn gerbil_utf8_comparison_snapshot(slot: i32);
    fn gerbil_utf8_comparison_stat(index: i32) -> f64;
}

struct Root(GerbilRootId);
impl Drop for Root {
    fn drop(&mut self) {
        // SAFETY: every root was acquired and remains on this runtime's owner.
        assert_eq!(
            unsafe { gerbil_scheme_sys::gerbil_scheme_rust_root_release(self.0) },
            GerbilStatus::Ok
        );
    }
}

fn convert(value: &Root, mode: i32) -> String {
    if mode == 4 {
        return buffer::convert(value);
    }
    if mode == 5 {
        return buffer::production(value);
    }
    // SAFETY: the preflighted mode and live string root belong to this owner.
    let raw = unsafe { gerbil_utf8_comparison_encode(value.0.0, mode) };
    assert!(
        raw > 0,
        "encoding control must fail closed, never publish root zero"
    );
    let root = Root(GerbilRootId(raw));
    let mut size = 0;
    // SAFETY: output is writable and root stays live throughout all operations.
    assert_eq!(
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_bytevector_length(root.0, &raw mut size)
        },
        GerbilStatus::Ok
    );
    let mut bytes = Vec::<u8>::with_capacity(size);
    // SAFETY: capacity admits exactly size bytes; the ABI initializes all of
    // them on Ok. No Scheme pointer is exposed or retained in Rust.
    assert_eq!(
        unsafe {
            gerbil_scheme_sys::gerbil_scheme_rust_root_bytevector_copy(
                root.0,
                bytes.as_mut_ptr(),
                size,
            )
        },
        GerbilStatus::Ok
    );
    // SAFETY: only successful, complete bulk initialization admits the length.
    unsafe { bytes.set_len(size) };
    simdutf8::basic::from_utf8(&bytes).expect("complete UTF-8 validation");
    // SAFETY: validation covered the entire exclusively owned buffer above.
    unsafe { String::from_utf8_unchecked(bytes) }
}

fn median(values: &[f64]) -> f64 {
    assert_eq!(values.len(), 20, "all samples must complete");
    assert!(
        values.iter().all(|value| value.is_finite()),
        "unresolved statistic"
    );
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[9].midpoint(sorted[10])
}

fn compare(values: &[Root], texts: &[String], jobs: usize, left: i32, right: i32) -> f64 {
    let mut wall = [Vec::new(), Vec::new()];
    let mut cpu = [Vec::new(), Vec::new()];
    let mut gc_cpu = [Vec::new(), Vec::new()];
    let mut gc_count = [0.0; 2];
    let mut paired = Vec::new();
    for group in 0..10 {
        let mut group_times = [0.0; 4];
        // ABBA yields 20 samples per variant and balances chronology/GC carry.
        for (position, slot) in [0, 1, 1, 0].into_iter().enumerate() {
            let mode = if slot == 0 { left } else { right };
            // SAFETY: diagnostic snapshots are owner-local, outside timing.
            unsafe { gerbil_utf8_comparison_snapshot(0) };
            let start = Instant::now();
            let mut total = 0;
            for index in 0..jobs {
                let text_slot = index % values.len();
                let text = convert(&values[text_slot], mode);
                assert_eq!(text, texts[text_slot], "changing text exact content");
                total += std::hint::black_box(text).len();
            }
            let elapsed = start.elapsed().as_secs_f64();
            assert!(
                elapsed.is_finite() && elapsed > 0.0,
                "unresolved sample clock"
            );
            assert_eq!(total, jobs * texts[0].len());
            // SAFETY: only this owner reads the completed sample's stats.
            unsafe { gerbil_utf8_comparison_snapshot(1) };
            let read_stats = |index| unsafe { gerbil_utf8_comparison_stat(index) };
            let process_cpu = read_stats(0) + read_stats(1);
            let collection_cpu = read_stats(3) + read_stats(4);
            let collections = read_stats(6);
            assert!(
                [process_cpu, collection_cpu, collections]
                    .into_iter()
                    .all(|value| value.is_finite() && value >= 0.0),
                "invalid cost counter"
            );
            wall[slot].push(elapsed);
            cpu[slot].push(process_cpu);
            gc_cpu[slot].push(collection_cpu);
            gc_count[slot] += collections;
            group_times[position] = elapsed;
            eprintln!(
                "UTF8-MATCHED jobs={jobs} left={left} right={right} group={group} position={position} mode={mode} completed={} elapsed_ms={:.3}",
                wall[slot].len(),
                elapsed * 1000.0
            );
        }
        paired.push(group_times[1] / group_times[0]);
        paired.push(group_times[2] / group_times[3]);
    }
    for slot in 0..2 {
        assert_eq!(wall[slot].len(), 20);
        println!(
            "UTF8-MATCHED-SUMMARY jobs={jobs} mode={} samples=20 wall_median_ms={:.3} cpu_median_ms={:.3} gc_cpu_median_ms={:.3} gc_count_total={}",
            if slot == 0 { left } else { right },
            median(&wall[slot]) * 1000.0,
            median(&cpu[slot]) * 1000.0,
            median(&gc_cpu[slot]) * 1000.0,
            gc_count[slot]
        );
    }
    println!(
        "UTF8-MATCHED-DECISION jobs={jobs} left={left} right={right} paired_ratio_median={:.6} slower={}",
        median(&paired),
        median(&paired) > 1.0
    );
    median(&paired)
}

fn main() {
    let argument = std::env::args().nth(1);
    let direct = argument.as_deref() == Some("--buffer");
    let contracts_only = argument.as_deref() == Some("--contracts");
    let requested = argument
        .filter(|_| !direct && !contracts_only)
        .map(|value| {
            let jobs: usize = value.parse().expect("load must be a number");
            assert!([1_000, 10_000, 100_000].contains(&jobs), "unsupported load");
            jobs
        });
    let program = gerbil_scheme_qualification::linked_program();
    let runtime = GerbilRuntime::initialize_program(program).expect("one owner-local AOT runtime");
    if direct || contracts_only {
        buffer::contracts(&runtime);
        if contracts_only {
            return;
        }
    }
    let texts: Vec<_> = ["a\0汉字😀", "b\0漢語🚀", "c\0中文🌍", "d\0文字🎉"]
        .into_iter()
        .map(|text| text.repeat(683))
        .collect();
    let values: Vec<_> = texts
        .iter()
        .map(|text| buffer::root_text(&runtime, text))
        .collect();
    for mode in 0..3 {
        for (value, text) in values.iter().zip(&texts) {
            assert_eq!(convert(value, mode), *text);
        }
    }
    let mut admitted = true;
    for jobs in [1_000, 10_000, 100_000] {
        if requested.is_some_and(|load| load != jobs) {
            continue;
        }
        if direct {
            admitted &= compare(&values, &texts, jobs, 3, 5) <= 1.0;
        } else {
            compare(&values, &texts, jobs, 0, 1);
            compare(&values, &texts, jobs, 1, 2);
        }
    }
    assert_eq!(runtime.add_i64(40, 2).unwrap(), 42);
    // Collect every load before rejecting; never hide a slower negative sample.
    assert!(
        admitted,
        "caller-buffer candidate is slower; retain the baseline"
    );
}
