use std::path::PathBuf;
use std::io::Error;


pub fn list_file(path: &str) -> Result<Vec<PathBuf>,Error> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            files.push(path);
        }
    }
    Ok(files)
}

/// 格式化时长（秒）为 mm:ss 格式
pub fn format_duration(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{}:{:02}", mins, secs)
}
