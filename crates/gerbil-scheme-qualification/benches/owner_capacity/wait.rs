//! Opt-in native wait trace controls, outside the timed workload.
use super::{GATE, measurement::Case};
use gerbil_scheme::instances::InstancePool;

pub(super) fn control(pool: &InstancePool, command: &[i64]) -> Vec<Vec<i64>> {
    (0..4)
        .map(|owner| {
            let command = command.to_vec();
            pool.dispatcher()
                .submit_native_on(owner, move |session| {
                    let input = session.numeric_vector(&command)?;
                    let output = session.call(&input)?;
                    session.copy_numeric::<i64>(&output)
                })
                .unwrap()
                .wait(GATE)
                .unwrap()
        })
        .collect()
}

pub(super) fn snapshot(pool: &InstancePool, inputs: &[Vec<i64>]) -> Vec<Vec<i64>> {
    let events = control(pool, &[11_i64]);
    let mut expected = [0_usize; 4];
    for (index, input) in inputs.iter().enumerate() {
        expected[index / 32 % 4] += usize::from(input[1] == 1);
    }
    for (owner, trace) in events.iter().enumerate() {
        assert_eq!(&trace[..2], &[0, 0], "native wait trace overflow/error");
        assert_eq!((trace.len() - 2) % 4, 0);
        let mut sleeps = 0;
        let mut selects = 0;
        for row in trace[2..].chunks_exact(4) {
            assert!((1..=5).contains(&row[0]));
            assert!(row[1] > 0 && row[2] >= row[1] && row[3] >= 0);
            if row[0] == 2 {
                sleeps += 1;
                assert_eq!(row[3], 1_000_000);
            } else {
                selects += 1;
            }
        }
        assert_eq!(sleeps, expected[owner], "all real Scheme waits observed");
        assert!(
            expected[owner] == 0 || selects > 0,
            "SDK select not observed"
        );
    }
    events
}

// Diagnostic pair: same native producer, only explicit owner OS QoS differs.
pub(super) const QOS_PAIRS: [[Case; 2]; 1] = [[
    Case {
        capacity: 4,
        work: 8,
        batch: 8,
        producer: true,
        transfer_control: None,
        owner_qos: Some(false),
    },
    Case {
        capacity: 4,
        work: 8,
        batch: 8,
        producer: true,
        transfer_control: None,
        owner_qos: Some(true),
    },
]];

pub(super) fn policy(pool: &InstancePool, selected: Option<bool>) -> Vec<Vec<i64>> {
    let Some(selected) = selected else {
        return Vec::new();
    };
    let receipts = control(pool, &[12_i64, i64::from(selected)]);
    assert!(
        receipts
            .iter()
            .all(|row| row[0] == 0 && (selected || row[1] == row[2])),
        "owner QoS selection/restoration failed: {receipts:?}"
    );
    receipts
}

pub(super) fn begin(pool: &InstancePool) -> bool {
    let enabled = std::env::args().any(|arg| arg == "--wait-trace");
    if enabled {
        control(pool, &[10_i64, 1]);
    }
    enabled
}
