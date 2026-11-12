//! Shared native Scheme order corpus and independent Rust result oracle.
fn integer(value: usize) -> i64 {
    i64::try_from(value).expect("bounded qualification integer")
}
pub(crate) fn order(id: usize, mixed: bool) -> Vec<i64> {
    let mut input = vec![integer(id), i64::from(mixed && id.rem_euclid(7) == 0)];
    for line in 0..48 {
        input.extend([
            integer((id + line) % 16),
            integer(line % 5 + 1),
            integer((id * 17 + line * 31) % 10000),
        ]);
    }
    input
}
pub(crate) fn verify(input: &[i64], output: &[i64]) {
    assert_eq!(output.len(), 21);
    assert_eq!(output[0], input[0]);
    let mut expected = [0_i64; 18];
    for line in input[2..].chunks_exact(3) {
        expected[0] += line[1];
        expected[1] += line[1] * line[2];
        expected[2 + usize::try_from(line[0]).unwrap()] += line[1] * line[2];
    }
    assert_eq!(output[1..3], expected[..2]);
    assert_eq!(output[5..], expected[2..]);
}
pub(crate) fn peers(output: &[Vec<i64>]) -> usize {
    if output.len() < 4 {
        return 0;
    }
    let mut ranks: Vec<_> = output.iter().map(|row| row[3]).collect();
    ranks.sort_unstable();
    assert_eq!(ranks, (1..=integer(output.len())).collect::<Vec<_>>());
    for (index, row) in output.iter().enumerate() {
        assert_eq!(row[4], integer(index % 4));
    }
    let first_finish = (0..4)
        .map(|worker| {
            output
                .iter()
                .filter(|row| row[4] == worker)
                .map(|row| row[3])
                .max()
                .unwrap()
        })
        .min()
        .unwrap();
    (0..4)
        .map(|worker| {
            output
                .iter()
                .filter(|row| row[4] == worker && row[3] <= first_finish)
                .count()
        })
        .min()
        .unwrap()
}
pub(crate) fn percentile(values: &mut [u128], percent: usize) -> u128 {
    values.sort_unstable();
    values[(values.len() * percent).div_ceil(100).saturating_sub(1)]
}
