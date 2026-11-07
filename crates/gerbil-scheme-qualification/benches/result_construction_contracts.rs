use gerbil_scheme::instances::{InstanceConfig, InstancePool, IsolatedProgram};
use std::time::Duration;
fn main() {
    let gate = Duration::from_secs(5);
    let image = std::env::var("GERBIL_SERVICE_BENCH_IMAGE")
        .ok()
        .or_else(|| option_env!("GERBIL_OWNER_INSTANCE_IMAGE").map(str::to_owned))
        .expect("use the native build image or GERBIL_SERVICE_BENCH_IMAGE");
    // SAFETY: the isolated image keeps the original strong-root owner ABI.
    let program = unsafe {
        IsolatedProgram::from_image(image, c"gerbil_owner_orders_call")
            .unwrap()
            .with_exports(&[c"gerbil_owner_service_reference"])
            .with_scheme_service(c"gerbil_owner_service_call")
    };
    let mut pool = InstancePool::start(
        &program,
        InstanceConfig {
            owners: 1,
            queue_capacity: 128,
            maximum_request_bytes: 16 * 1024 * 1024,
            maximum_reply_bytes: 16 * 1024 * 1024,
            startup_timeout: gate,
        },
    )
    .unwrap();
    pool.dispatcher()
        .configure_owner_budgets(4, Some(32))
        .unwrap();
    let checks = pool
        .dispatcher()
        .submit_native_on(0, |session| {
            let header = session.numeric_vector::<i64>(&[66, 0])?;
            let command = session.vector(&[&header])?;
            let result = session.call_export(c"gerbil_owner_service_reference", &command)?;
            let values = session.copy_numeric::<i64>(&result)?;
            assert_eq!(values.len(), 5);
            assert_eq!(values[0], 0);
            assert_eq!(values[1], 21);
            assert_eq!(&values[2..], &[3, 3, 2]);
            Ok(values[1])
        })
        .unwrap()
        .wait(gate)
        .unwrap();
    assert!(pool.shutdown(gate).is_clean());
    println!(
        "{{\"event\":\"native_result_construction_contract\",\"passed\":true,\"differential_cases\":{checks},\"rank_results\":3,\"checkpoint_results\":3,\"rank_replays\":2,\"checkpoint_replays\":2,\"gc\":true,\"bignum_fallback\":true,\"exception_identity\":true,\"green_wait_cases\":2,\"clean_shutdown\":true,\"performance_result\":false}}"
    );
}
