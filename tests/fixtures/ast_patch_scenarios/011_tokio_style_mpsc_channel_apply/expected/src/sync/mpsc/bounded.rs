pub(crate) struct Sender<T> {
    value: Option<T>,
}

pub(crate) struct Receiver<T> {
    value: Option<T>,
}

pub(crate) fn channel<T>(buffer: usize) -> (Sender<T>, Receiver<T>)
where
    T: Send + 'static,
{
    assert!(buffer > 0, "mpsc channel buffer must be positive");
    let sender = Sender { value: None };
    let receiver = Receiver { value: None };
    (sender, receiver)
}
