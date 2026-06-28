//! Expected fixture keeps exact I/O outside cancellation competition.

use tokio::io::{AsyncRead, AsyncReadExt};

pub async fn read_frame<R>(
    reader: &mut R,
    mut shutdown: tokio::sync::oneshot::Receiver<()>,
) -> std::io::Result<Option<[u8; 8]>>
where
    R: AsyncRead + Unpin,
{
    if shutdown.try_recv().is_ok() {
        return Ok(None);
    }

    let mut buf = [0; 8];
    reader.read_exact(&mut buf).await?;
    Ok(Some(buf))
}
