use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

const MAX_LOG_LINES: usize = 1000;

#[derive(Debug, Clone)]
pub struct ServiceLogger {
    logs: Arc<Mutex<VecDeque<String>>>,
}

impl ServiceLogger {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOG_LINES))),
        }
    }

    pub fn log(&self, message: String) {
        let mut logs = self.logs.lock().unwrap();
        if logs.len() >= MAX_LOG_LINES {
            logs.pop_front();
        }
        logs.push_back(message);
    }

    pub fn get_logs(&self, lines: usize) -> Vec<String> {
        let logs = self.logs.lock().unwrap();
        let start = if logs.len() > lines {
            logs.len() - lines
        } else {
            0
        };
        logs.iter().skip(start).cloned().collect()
    }

    pub fn clear(&self) {
        let mut logs = self.logs.lock().unwrap();
        logs.clear();
    }
}

pub struct Logger;

impl Logger {
    pub fn init() {
        // Simple stderr logger since we're init
    }

    pub fn info(msg: &str) {
        let timestamp = Self::timestamp();
        eprintln!("[{}] [INFO] {}", timestamp, msg);
    }

    pub fn warn(msg: &str) {
        let timestamp = Self::timestamp();
        eprintln!("[{}] [WARN] {}", timestamp, msg);
    }

    pub fn error(msg: &str) {
        let timestamp = Self::timestamp();
        eprintln!("[{}] [ERROR] {}", timestamp, msg);
    }

    pub fn debug(msg: &str) {
        let timestamp = Self::timestamp();
        eprintln!("[{}] [DEBUG] {}", timestamp, msg);
    }

    fn timestamp() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        format!("{}", now)
    }
}
