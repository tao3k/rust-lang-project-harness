//! Input fixture with cancellation-unsafe exact I/O inside `tokio::time::timeout`.

use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};

pub async fn read_frame<R>(reader: &mut R) -> std::io::Result<[u8; 8]>
where
    R: AsyncRead + Unpin,
{
    let mut buf = [0; 8];
    tokio::time::timeout(Duration::from_secs(1), reader.read_exact(&mut buf)).await??;
    Ok(buf)
}
