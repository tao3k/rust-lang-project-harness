use std::time::Duration;

pub async fn summarize(paths: Vec<String>) -> usize {
    let mut total = 0;
    for path in paths {
        std::thread::sleep(Duration::from_millis(2));
        total += path.len();
    }
    total
}
