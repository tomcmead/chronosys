mod system_metrics;

fn main() {
    // Setup env_logger to read from the RUST_LOG env variable
    #[cfg(feature = "logging")]
    {
        let env = env_logger::Env::default().default_filter_or("info");
        env_logger::Builder::from_env(env).init();
        log::debug!("Log Level: {:?}", log::max_level());
    }

    log::debug!("Chronosys starting...");

    let mut global_metrics = match system_metrics::GlobalMetricsCollector::new() {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to initialise GlobalMetricsCollector: {e:?}");
            return;
        }
    };

    match global_metrics.get_metrics() {
        Ok(metrics) => log::debug!("Memory Metrics: {0:?}", metrics.memory),
        Err(e) => log::error!("Error collecting global metrics: {e:?}"),
    }

    log::debug!("Chronosys exited");
}
