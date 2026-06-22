fn main() {
    // Setup env_logger to read from the RUST_LOG env variable
    #[cfg(feature = "logging")]
    {
        let env = env_logger::Env::default().default_filter_or("info");
        env_logger::Builder::from_env(env).init();
        log::debug!("Log Level: {:?}", log::max_level());
    }
}