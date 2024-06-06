use std::time::SystemTime;
use chrono::{Local, Utc};

/// 返回 UTC 时间戳
pub fn time() -> u32 {
    Utc::now().timestamp() as u32
}

/// 格式化时间戳
pub fn format_time(time: u32) -> String {

    let system_time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(time as u64);
    let utc_time = chrono::DateTime::<Utc>::from(system_time);
    let local_time = utc_time.with_timezone(&Local);

    local_time.format("%Y年%m月%d日 %H:%M:%S").to_string()
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_time() {
        let t = time();
        println!("当前时间戳: {}", t);
        println!("格式化时间: {}", format_time(t));
    }
}