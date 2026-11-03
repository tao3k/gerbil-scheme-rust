//! Cold reduction of the original microsecond histogram; no message-path work.
pub fn summarize(histogram: &[u64]) -> Result<[i64; 3], &'static str> {
    let samples = histogram.iter().try_fold(0_u64, |sum, n| sum.checked_add(*n))
        .ok_or("terminal histogram sample overflow")?;
    let count = i64::try_from(samples).map_err(|_| "terminal sample count exceeds receipt type")?;
    if samples == 0 { return Ok([0, -1, -1]); }
    let rank99 = (u128::from(samples) * 99).div_ceil(100);
    let rank999 = (u128::from(samples) * 999).div_ceil(1000);
    let (mut cumulative, mut p99, mut p999) = (0_u128, None, None);
    for (bucket, n) in histogram.iter().enumerate() {
        cumulative += u128::from(*n);
        let upper = || i64::try_from(bucket + 1).ok().and_then(|n| n.checked_mul(1000))
            .ok_or("terminal histogram bucket overflow");
        if p99.is_none() && cumulative >= rank99 { p99 = Some(upper()?); }
        if cumulative >= rank999 { p999 = Some(upper()?); break; }
    }
    Ok([count, p99.ok_or("terminal p99 missing")?, p999.ok_or("terminal p999 missing")?])
}
#[cfg(test)]
mod tests {

    #[test] fn empty_and_unsampled() {
        assert_eq!(super::summarize(&[]), Ok([0,-1,-1]));
        assert_eq!(super::summarize(&[0;4]), Ok([0,-1,-1]));
    }
    #[test] fn original_upper_bucket_semantics() {
        assert_eq!(super::summarize(&[0,0,1,0]), Ok([1,3000,3000]));
        assert_eq!(super::summarize(&[99,0,1]), Ok([100,1000,3000]));
    }
    #[test] fn non_integer_ranks_round_up() {
        assert_eq!(super::summarize(&[99,1,1]), Ok([101,2000,3000]));
        assert_eq!(super::summarize(&[999,1,1]), Ok([1001,1000,2000]));
    }
    #[test] fn overflowing_counts_fail() {
        assert!(super::summarize(&[u64::MAX,1]).is_err());
        assert!(super::summarize(&[i64::MAX as u64+1]).is_err());
    }
    #[test] fn full_diagnostic_storage_preserves_ranks() {
        let mut bins=vec![0;10_000_000]; bins[9_999_999]=1;
        assert_eq!(super::summarize(&bins),Ok([1,10_000_000_000,10_000_000_000]));
    }
}
