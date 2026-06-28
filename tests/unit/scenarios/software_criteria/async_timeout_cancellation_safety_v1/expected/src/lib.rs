//! Expected fixture keeps exact I/O progress outside timeout cancellation.

use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};

pub async fn read_frame<R>(reader: &mut R) -> std::io::Result<[u8; 8]>
where
    R: AsyncRead + Unpin,
{
    tokio::time::timeout(Duration::from_secs(1), async { Ok::<(), std::io::Error>(()) }).await??;
    let mut buf = [0; 8];
    reader.read_exact(&mut buf).await?;
    Ok(buf)
}
