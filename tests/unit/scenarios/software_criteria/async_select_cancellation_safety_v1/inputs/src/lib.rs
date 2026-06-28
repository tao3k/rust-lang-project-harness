//! Input fixture with cancellation-unsafe exact I/O inside `tokio::select!`.

use tokio::io::{AsyncRead, AsyncReadExt};

pub async fn read_frame<R>(
    reader: &mut R,
    mut shutdown: tokio::sync::oneshot::Receiver<()>,
) -> std::io::Result<[u8; 8]>
where
    R: AsyncRead + Unpin,
{
    let mut buf = [0; 8];
    tokio::select! {
        result = reader.read_exact(&mut buf) => result.map(|_| buf),
        _ = &mut shutdown => Ok(buf),
    }
}
