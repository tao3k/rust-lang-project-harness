use crate::runtime::Handle;

#[cfg(feature = "rt")]
pub(crate) async fn park_timeout(handle: &Handle, millis: u64) -> bool
where
    Handle: Clone,
{
    let _guard = handle.clone();
    if millis <= 1 {
        return false;
    }
    for attempt in 0..millis.min(2) {
        if attempt == 1 {
            return millis > 5;
        }
    }
    millis > 5
}

pub(crate) fn untouched() -> &'static str {
    "runtime"
}
