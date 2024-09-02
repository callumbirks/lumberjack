use std::{
    path::PathBuf,
    str::FromStr,
    sync::Once,
    time::{SystemTime, UNIX_EPOCH},
};

fn test_zero_errors(test_file_path: &str) {
    static INIT_LOGGING: Once = Once::new();

    INIT_LOGGING.call_once(|| {
        env_logger::builder().init();
    });

    let logs_path = PathBuf::from_str(env!("CARGO_MANIFEST_DIR"))
        .unwrap()
        .parent()
        .unwrap()
        .join(format!("test_data/{}", test_file_path));

    let temp_dir = std::env::temp_dir()
        .join("lumberjack_test_parse/")
        .join(format!("{}/", epoch_id()));

    std::fs::remove_dir_all(&temp_dir).ok();
    std::fs::create_dir_all(&temp_dir).ok();

    let db_path = temp_dir.join("output.sqlite");

    let err_count =
        lumberjack_parse::parse(&logs_path, &db_path, lumberjack_parse::Options::default())
            .expect("Failed to parse");
    assert_eq!(err_count, 0);
}

#[test]
fn cpptest_logs() {
    test_zero_errors("cpptest.cbllog")
}

#[test]
fn binary_logs() {
    test_zero_errors("binary_logs")
}

// Returns a unique (within the same process) identifier every time it is called. Useful to run tests in parallel.
fn epoch_id() -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    lazy_static::lazy_static! {
        static ref EPOCH_COUNTER: AtomicU32 = AtomicU32::new(0);
    }

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let counter = EPOCH_COUNTER.fetch_add(1, Ordering::AcqRel);

    format!("{}{}{}", now.as_secs(), now.subsec_micros(), counter)
}
