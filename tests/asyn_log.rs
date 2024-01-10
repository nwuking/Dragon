use Dragon::asyn_log::{Logger, LogCfg};


#[test]
fn logger_init() {
    let mut log_cfg = LogCfg::new();
    log_cfg.enable_console = true;
    log_cfg.dir = String::from("./tests/");
    log_cfg.file_max_size = 1 * 1024 * 1024;
    log_cfg.file_max_count = 3;

    let cfg = Some(log_cfg);

    Logger::init(cfg).unwrap();

    for _ in 0..100 {
        log::info!(target: "DGTEST", "hello world");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    for _ in 0..100 {
        log::info!(target: "DGTEST", "hello nwuking");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}