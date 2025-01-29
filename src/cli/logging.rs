pub fn init_logging(level: log::LevelFilter) {
    let _ = env_logger::builder()
            .filter_level(level)
            .is_test(true)
            .init();
}
