use super::{GATE, Progress, config, drained, observed, program};
use gerbil_scheme::instances::InstancePool;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn corpus(rows: usize) -> (Vec<Vec<u8>>, Vec<Value>) {
    let mut requests = Vec::new();
    let mut expected = Vec::new();
    for salt in 0..8 {
        let mut input = Vec::new();
        let mut groups = BTreeMap::new();
        let mut total = 0;
        let mut characters = 0;
        for row in 0..rows {
            let category = ["alpha", "beta", "gamma"][row % 3];
            let quantity = (row + salt) % 17 + 1;
            let price = row % 97 + 3;
            let label = format!("order-{row}-λ-🧪\0");
            characters += label.chars().count();
            let amount = quantity * price;
            total += amount;
            *groups.entry(category).or_insert(0) += amount;
            input.push(json!([category, quantity, price, label]));
        }
        requests.push(serde_json::to_vec(&json!(["summary", input, true])).unwrap());
        expected.push(json!([
            total,
            rows,
            characters,
            groups
                .into_iter()
                .map(|(key, value)| json!([key, value]))
                .collect::<Vec<_>>()
        ]));
    }
    (requests, expected)
}
fn batch(pool: &InstancePool, requests: &[Vec<u8>], expected: &[Value], progress: &Progress) {
    let tickets: Vec<_> = requests
        .iter()
        .map(|request| pool.submit(request.clone()).expect("bounded admission"))
        .collect();
    for (ticket, expected) in tickets.iter().zip(expected) {
        let reply = ticket.wait(GATE).expect("Scheme summary");
        assert_eq!(
            &serde_json::from_slice::<Value>(&reply.bytes).expect("JSON response"),
            expected
        );
        observed(progress, "observed real Scheme workload response");
    }
}
fn sample(
    variant: usize,
    rows: usize,
    pool: &InstancePool,
    requests: &[Vec<u8>],
    expected: &[Value],
    progress: &Progress,
) -> u128 {
    let start = Instant::now();
    batch(pool, requests, expected, progress);
    let ns = start.elapsed().as_nanos();
    eprintln!(
        "LIBRARY-SAMPLE variant={variant} rows-per-request={rows} requests=8 ns={ns} validated=8"
    );
    ns
}
pub(super) fn cold(owners: usize, progress: &Progress) {
    let (requests, expected) = corpus(1024);
    let total = Instant::now();
    let setup = Instant::now();
    let mut pool = InstancePool::start(&program(), config(owners)).expect("cold native owners");
    let setup_ns = setup.elapsed().as_nanos();
    observed(progress, "observed cold pool initialized");
    let first = sample(
        usize::from(owners == 4),
        1024,
        &pool,
        &requests,
        &expected,
        progress,
    );
    let stop = Instant::now();
    drained(&mut pool, progress);
    eprintln!(
        "LIBRARY-COLD owners={owners} setup-ns={setup_ns} first-batch-ns={first} teardown-ns={} total-ns={} image-unload=false",
        stop.elapsed().as_nanos(),
        total.elapsed().as_nanos()
    );
}
fn cold_children(progress: &Progress) {
    for owners in [1, 4, 4, 1, 1, 4, 4, 1] {
        let process_start = Instant::now();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--cold", &owners.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("fresh cold process");
        let start = Instant::now();
        loop {
            if child.try_wait().expect("cold child status").is_some() {
                break;
            }
            if start.elapsed() >= Duration::from_secs(4) {
                child.kill().expect("kill stalled cold child");
                child.wait().expect("reap cold child");
                panic!("cold child exceeded four-second process gate; cleanup unverified");
            }
            thread::sleep(Duration::from_millis(5));
        }
        let output = child.wait_with_output().expect("cold child receipt");
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
        assert!(output.status.success());
        eprintln!(
            "LIBRARY-COLD-PROCESS owners={owners} wall-ns={}",
            process_start.elapsed().as_nanos()
        );
        observed(progress, "observed completed fresh-process cold run");
    }
}
pub(super) fn matched(smoke: bool, progress: &Progress) {
    let start = Instant::now();
    let mut a = InstancePool::start(&program(), config(1)).expect("baseline owner");
    let a_setup = start.elapsed().as_nanos();
    observed(progress, "observed baseline initialized");
    let start = Instant::now();
    let mut b = InstancePool::start(&program(), config(4)).expect("candidate owners");
    eprintln!(
        "LIBRARY-SETUP a-ns={a_setup} b-ns={}",
        start.elapsed().as_nanos()
    );
    observed(progress, "observed candidate initialized");
    let states: std::collections::BTreeSet<_> = a
        .owners()
        .iter()
        .chain(b.owners())
        .map(|owner| {
            assert_eq!(owner.processors, 1);
            eprintln!(
                "LIBRARY-OWNER owner={} state={:#x} processors={} setup-ns={}",
                owner.owner,
                owner.state,
                owner.processors,
                owner.setup_time.as_nanos()
            );
            owner.state
        })
        .collect();
    assert_eq!(states.len(), 5);
    for rows in [64, 1024, 8192] {
        let (requests, expected) = corpus(rows);
        batch(&a, &requests, &expected, progress);
        batch(&b, &requests, &expected, progress);
        let mut samples: [Vec<u128>; 2] = [Vec::new(), Vec::new()];
        for block in 0..if smoke { 1 } else { 10 } {
            for variant in [0, 1, 1, 0] {
                let pool = if variant == 0 { &a } else { &b };
                let ns = sample(variant, rows, pool, &requests, &expected, progress);
                samples[variant].push(ns);
                eprintln!("LIBRARY-BLOCK rows={rows} block={block} variant={variant}");
            }
        }
        for values in &mut samples {
            values.sort_unstable();
        }
        let median =
            |values: &[u128]| values[(values.len() - 1) / 2].midpoint(values[values.len() / 2]);
        eprintln!(
            "LIBRARY-RESULT rows-per-request={rows} requests=8 samples-per-variant={} a-median-ns={} b-median-ns={} workload=native-scheme-json-aggregate forced-gc-per-request=true",
            samples[0].len(),
            median(&samples[0]),
            median(&samples[1])
        );
    }
    drained(&mut a, progress);
    let (requests, expected) = corpus(64);
    batch(&b, &requests, &expected, progress);
    drained(&mut b, progress);
    if !smoke {
        cold_children(progress);
    }
}
