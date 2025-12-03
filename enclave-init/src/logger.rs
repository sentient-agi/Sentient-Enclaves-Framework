use anyhow::Result;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const MAX_MEMORY_LOG_LINES: usize = 100;

#[derive(Debug, Clone)]
pub struct ServiceLogger {
    log_file: Arc<Mutex<Option<File>>>,
    log_path: PathBuf,
    memory_logs: Arc<Mutex<Vec<String>>>,
    max_log_size: u64,
    max_log_files: usize,
}

impl ServiceLogger {
    pub fn new(
        log_dir: &str,
        service_name: &str,
        max_log_size: u64,
        max_log_files: usize,
    ) -> Result<Self> {
        // Create log directory if it doesn't exist
        fs::create_dir_all(log_dir)?;

        let log_path = Path::new(log_dir).join(format!("{}.log", service_name));

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        Ok(Self {
            log_file: Arc::new(Mutex::new(Some(file))),
            log_path,
            memory_logs: Arc::new(Mutex::new(Vec::with_capacity(MAX_MEMORY_LOG_LINES))),
            max_log_size,
            max_log_files,
        })
    }

    pub fn log(&self, message: String) {
        let timestamp = Self::timestamp();
        let formatted = format!("[{}] {}", timestamp, message);

        // Write to file
        if let Ok(mut file_opt) = self.log_file.lock() {
            if let Some(ref mut file) = *file_opt {
                let _ = writeln!(file, "{}", formatted);
                let _ = file.flush();

                // Check file size and rotate if needed
                if let Ok(metadata) = file.metadata() {
                    if metadata.len() > self.max_log_size {
                        drop(file_opt); // Release lock before rotating
                        let _ = self.rotate_logs();
                    }
                }
            }
        }

        // Keep last N lines in memory for quick access
        if let Ok(mut logs) = self.memory_logs.lock() {
            if logs.len() >= MAX_MEMORY_LOG_LINES {
                logs.remove(0);
            }
            logs.push(formatted);
        }
    }

    fn rotate_logs(&self) -> Result<()> {
        // Close current file
        if let Ok(mut file_opt) = self.log_file.lock() {
            *file_opt = None;
        }

        // Rotate existing log files
        for i in (1..self.max_log_files).rev() {
            let old_path = if i == 1 {
                self.log_path.clone()
            } else {
                self.log_path.with_extension(format!("log.{}", i - 1))
            };

            let new_path = self.log_path.with_extension(format!("log.{}", i));

            if old_path.exists() {
                let _ = fs::rename(&old_path, &new_path);
            }
        }

        // Delete oldest log file if we've reached the limit
        let oldest = self
            .log_path
            .with_extension(format!("log.{}", self.max_log_files));
        if oldest.exists() {
            let _ = fs::remove_file(oldest);
        }

        // Open new log file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        if let Ok(mut file_opt) = self.log_file.lock() {
            *file_opt = Some(file);
        }

        Ok(())
    }

    pub fn get_logs(&self, lines: usize) -> Vec<String> {
        // Try to read from file first
        if let Ok(content) = fs::read_to_string(&self.log_path) {
            let all_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
            let start = if all_lines.len() > lines {
                all_lines.len() - lines
            } else {
                0
            };
            return all_lines.into_iter().skip(start).collect();
        }

        // Fallback to memory logs
        if let Ok(logs) = self.memory_logs.lock() {
            let start = if logs.len() > lines {
                logs.len() - lines
            } else {
                0
            };
            return logs.iter().skip(start).cloned().collect();
        }

        Vec::new()
    }

    pub fn get_all_logs(&self) -> Result<String> {
        let mut all_content = String::new();

        // Read rotated logs first (oldest to newest)
        for i in (1..self.max_log_files).rev() {
            let rotated_path = self.log_path.with_extension(format!("log.{}", i));
            if rotated_path.exists() {
                if let Ok(content) = fs::read_to_string(&rotated_path) {
                    all_content.push_str(&content);
                }
            }
        }

        // Read current log
        if self.log_path.exists() {
            if let Ok(content) = fs::read_to_string(&self.log_path) {
                all_content.push_str(&content);
            }
        }

        Ok(all_content)
    }

    pub fn clear(&self) -> Result<()> {
        // Close file
        if let Ok(mut file_opt) = self.log_file.lock() {
            *file_opt = None;
        }

        // Remove log files
        let _ = fs::remove_file(&self.log_path);
        for i in 1..self.max_log_files {
            let rotated_path = self.log_path.with_extension(format!("log.{}", i));
            let _ = fs::remove_file(rotated_path);
        }

        // Clear memory logs
        if let Ok(mut logs) = self.memory_logs.lock() {
            logs.clear();
        }

        // Reopen log file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        if let Ok(mut file_opt) = self.log_file.lock() {
            *file_opt = Some(file);
        }

        Ok(())
    }

    fn timestamp() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Format as human-readable timestamp
        let naive = chrono::NaiveDateTime::from_timestamp_opt(now as i64, 0)
            .unwrap_or_default();
        naive.format("%Y-%m-%d %H:%M:%S").to_string()
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

        let naive = chrono::NaiveDateTime::from_timestamp_opt(now as i64, 0)
            .unwrap_or_default();
        naive.format("%Y-%m-%d %H:%M:%S").to_string()
    }
}

// Simple chrono replacement for timestamp formatting
mod chrono {
    pub struct NaiveDateTime {
        timestamp: i64,
    }

    impl NaiveDateTime {
        pub fn from_timestamp_opt(secs: i64, _nsecs: u32) -> Option<Self> {
            Some(Self { timestamp: secs })
        }

        pub fn format(&self, _fmt: &str) -> FormattedTime {
            FormattedTime {
                timestamp: self.timestamp,
            }
        }
    }

    impl Default for NaiveDateTime {
        fn default() -> Self {
            Self { timestamp: 0 }
        }
    }

    pub struct FormattedTime {
        timestamp: i64,
    }

    impl std::fmt::Display for FormattedTime {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            // Simple UTC formatting
            let secs = self.timestamp;
            let days = secs / 86400;
            let rem_secs = secs % 86400;
            let hours = rem_secs / 3600;
            let minutes = (rem_secs % 3600) / 60;
            let seconds = rem_secs % 60;

            // Calculate date (simplified, starting from 1970-01-01)
            let mut year = 1970;
            let mut remaining_days = days;

            loop {
                let days_in_year = if is_leap_year(year) { 366 } else { 365 };
                if remaining_days < days_in_year {
                    break;
                }
                remaining_days -= days_in_year;
                year += 1;
            }

            let (month, day) = day_to_month_day(remaining_days, is_leap_year(year));

            write!(
                f,
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                year, month, day, hours, minutes, seconds
            )
        }
    }

    fn is_leap_year(year: i64) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }

    fn day_to_month_day(day_of_year: i64, is_leap: bool) -> (u8, u8) {
        let days_in_month = if is_leap {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };

        let mut remaining = day_of_year;
        for (month_idx, &days) in days_in_month.iter().enumerate() {
            if remaining < days as i64 {
                return ((month_idx + 1) as u8, (remaining + 1) as u8);
            }
            remaining -= days as i64;
        }
        (12, 31)
    }
}
