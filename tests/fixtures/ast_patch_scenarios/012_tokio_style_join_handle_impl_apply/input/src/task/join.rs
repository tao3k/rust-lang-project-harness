use core::task::{Context, Poll};

pub(crate) struct JoinHandle<T> {
    output: Option<T>,
}

pub(crate) struct JoinError;

impl<T> JoinHandle<T>
where
    T: Send + 'static,
{
    pub(crate) fn poll_join(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<T, JoinError>> {
        match self.output.take() {
            Some(value) => Poll::Ready(Ok(value)),
            None => Poll::Pending,
        }
    }
}
