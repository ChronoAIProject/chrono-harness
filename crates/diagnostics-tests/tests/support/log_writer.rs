use chrono_diagnostics::{
    logging::{Level, Logger},
    rolling::RollingFile,
};
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let writer = RollingFile::open(std::path::Path::new(&args[0]), 2048, 64).unwrap();
    let logger = Logger::new(&args[1], Level::INFO, writer);
    logger.run(|| {
        for sequence in 0..40 {
            tracing::info!(event_id = "fixture.record", sequence, "concurrent record");
        }
    });
    logger.finish().unwrap();
}
