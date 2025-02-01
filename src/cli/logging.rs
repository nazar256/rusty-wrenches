pub fn init_logging(level: log::LevelFilter) {
    env_logger::builder()
        .filter_level(level)
        .is_test(true)
        .init();
}
