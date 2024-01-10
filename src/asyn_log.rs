use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    io::{stderr, stdout, Write},
    path,
    sync::{mpsc, Once},
    thread,
};

pub struct Logger;
impl Logger {
    pub fn init(cfg: Option<LogCfg>) -> Result<(), log::SetLoggerError> {
        static ONCE_INIT: Once = Once::new();
        let mut ret: Result<(), log::SetLoggerError> = Ok(());
        ONCE_INIT.call_once(|| {
            static mut LOGGER: LoggerInner = LoggerInner::new();

            unsafe {
                cfg.map(|config| {
                    LOGGER.cfg = config.clone();
                });

                let _ = LOGGER.start();
                ret = log::set_logger(&LOGGER).map(|()| log::set_max_level(LOGGER.cfg.level));

                let (tx, rx) = mpsc::sync_channel::<String>(1024);
                LOGGER.tx = Some(tx);

                thread::spawn(move || {
                    LOGGER.run(rx);
                });
            }
        });

        ret
    }
}

#[derive(Clone)]
pub struct LogCfg {
    // TODO
    level: log::LevelFilter,
    enable_console: bool,
    dir: String,
    file_max_size: usize,
    file_max_count: usize,
}

struct LoggerInner {
    // TODO
    cfg: LogCfg,
    start: bool,
    file_handler: Option<fs::File>,
    file_bytes: usize,
    tx: Option<mpsc::SyncSender<String>>,
}
// unsafe impl Sync for LoggerInner {}

impl log::Log for LoggerInner {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn flush(&self) {
        // TODO
    }

    fn log(&self, record: &log::Record) {
        // 获取当前时间
        let now = chrono::Local::now();

        // 获取当前线程号
        // TODO thread::current().id().as_u64() 是unstable的
        // TODO 用loacl_threa_id记录线程号？
        let thread_id = thread::current().id();
        let mut hasher = DefaultHasher::new();
        thread_id.hash(&mut hasher);
        let tid = hasher.finish();

        static LOG_LEVEL_STR: [&str; 6] = ["None", "E", "W", "I", "D", "T"];

        // 年-月-日 时:分:秒.毫秒格式化时间
        let now = now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();

        // 格式化日志
        let msg = format!(
            "{} {} [{}]-[{}] {}\n",
            now,
            tid,
            LOG_LEVEL_STR[record.level() as usize],
            record.target(),
            record.args()
        );
        if self.cfg.enable_console {
            // TODO 区分platform 打印到console
            write!(stdout(), "{}", msg).unwrap();
        }

        if let Some(tx) = &self.tx {
            let _ = match tx.send(msg) {
                _ => {}
            };
        }
    }
}

impl LoggerInner {
    const fn new() -> Self {
        Self {
            cfg: LogCfg {
                level: log::LevelFilter::Info,
                enable_console: false,
                dir: String::new(),
                file_max_size: 0,
                file_max_count: 0,
            },
            start: false,
            file_handler: None,
            file_bytes: 0,
            tx: None,
        }
    }

    fn start(&mut self) -> Result<(), String> {
        // TODO 初始化配置
        if !self.cfg.dir.is_empty() {
            // 检查目录是否存在，不存在则创建
            let _ = path::Path::new(&self.cfg.dir).exists() || {
                // 语法糖，false则执行
                match fs::create_dir_all(&self.cfg.dir) {
                    Ok(_) => {}
                    Err(e) => {
                        Err(format!("create dir error: {}", e))?;
                    }
                }
                true
            };

            let _ = self.cfg.dir.ends_with("/") || {
                self.cfg.dir.push_str("/");
                true
            };

            // 检查目录是否可写
            self.file_handler = match fs::OpenOptions::new()
                .write(true)
                .create(true)
                .append(true)
                .open(format!("{}dragon.log.0", self.cfg.dir))
            {
                Ok(file) => {
                    let meta = file.metadata();
                    self.file_bytes = match meta {
                        Ok(meta) => meta.len() as usize,
                        Err(e) => {
                            let _ = writeln!(stderr(), "get file meta error: {}", e);
                            0
                        }
                    };
                    self.roll_file()
                }

                Err(e) => {
                    let _ = writeln!(stderr(), "open dir error: {}", e);
                    None
                }
            }
        }

        Ok(())
    }

    fn run(&mut self, rx: mpsc::Receiver<String>) {
        // self.op = true;
        while self.start {
            match rx.recv() {
                Ok(msg) => {
                    // 写到文件
                    if let Some(file) = &mut self.file_handler {
                        match file.write_all(msg.as_bytes()) {
                            Ok(_) => {
                                self.file_bytes += msg.len();
                            }
                            Err(e) => {
                                let _ = writeln!(stderr(), "write file error: {}", e);
                                // break;
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = writeln!(stderr(), "recv error: {}", e);
                    break;
                }
            }
        }
    }

    fn _stop(&mut self) {
        self.start = false;
    }

    fn roll_file(&mut self) -> Option<fs::File> {
        // TODO
        let mut ff = None;
        if self.file_bytes >= self.cfg.file_max_size {
            for i in (0..self.cfg.file_max_count - 1).rev() {
                let src = format!("{}dragon.log.{}", self.cfg.dir, i);
                // 判断源文件是否存在
                if !path::Path::new(&src).exists() {
                    continue;
                }

                let dst = format!("{}dragon.log.{}", self.cfg.dir, i + 1);
                match fs::rename(src, dst) {
                    Ok(_) => {
                        // 打开文件
                        match fs::OpenOptions::new()
                            .write(true)
                            .create(true)
                            .append(true)
                            .open(format!("{}dragon.log.0", self.cfg.dir))
                        {
                            Ok(file) => {
                                ff = Some(file);
                                self.file_bytes = 0;
                            }
                            Err(e) => {
                                let _ = writeln!(stderr(), "open file error: {}", e);
                                // None?
                            }
                        };
                    }
                    Err(e) => {
                        let _ = writeln!(stderr(), "rename file error: {}", e);
                        // None?
                    }
                }
            }
        }
        ff
    }
}

// 测试私有函数
#[cfg(test)]
mod tests {
    use std::time::Duration;

    use log::Log;

    use super::*;

    #[test]
    fn test_start() {
        let mut logger = LoggerInner::new();
        logger.cfg.dir = String::from("./tests/");
        logger.cfg.file_max_size = 10;
        logger.cfg.file_max_count = 3;
        logger.start().unwrap();
    }

    #[test]
    fn test_roll_file() {
        let mut logger = LoggerInner::new();
        logger.cfg.dir = String::from("./tests/");
        logger.cfg.file_max_size = 10;
        logger.cfg.file_max_count = 3;
        logger.file_handler = Some(fs::File::create("./tests/dragon.log.0").unwrap());

        for _ in 0..4 {
            logger.file_bytes = 10;
            logger.roll_file();
            assert_eq!(logger.file_bytes, 0);
        }
    }

    #[test]
    fn test_run() {
        let mut logger = LoggerInner::new();
        logger.cfg.dir = String::from("./tests");
        logger.cfg.file_max_size = 10;
        logger.cfg.file_max_count = 3;
        logger.file_handler = Some(fs::File::create("./tests/dragon.log.0").unwrap());
        logger.start = true;

        let (tx, rx) = mpsc::sync_channel::<String>(1024);

        let _ = thread::Builder::new()
            .name("test_run".to_string())
            .spawn(move || {
                logger.run(rx);
            })
            .unwrap();

        thread::sleep(Duration::from_millis(100));

        tx.send(String::from("hello\n")).unwrap();
        tx.send(String::from("world\n")).unwrap();

        thread::sleep(Duration::from_millis(100));
        tx.send(String::from("nwuking\n")).unwrap();
        tx.send(true.to_string()).unwrap();

        thread::sleep(Duration::from_millis(100));
    }

    #[test]
    fn test_log() {
        let mut logger = LoggerInner::new();
        logger.cfg.dir = String::from("./tests");
        logger.cfg.file_max_size = 10;
        logger.cfg.file_max_count = 3;
        logger.cfg.enable_console = true;

        let record = log::Record::builder()
            .args(format_args!("hello"))
            .level(log::Level::Info)
            .target("test_log")
            .file(Some("test_log"))
            .line(Some(1))
            .module_path(Some("test_log"))
            .build();
        logger.log(&record);

        let record = log::Record::builder()
            .args(format_args!("world"))
            .level(log::Level::Error)
            .target("test_log")
            .file(Some("test_log"))
            .line(Some(1))
            .module_path(Some("test_log"))
            .build();
        logger.log(&record);
    }
}
