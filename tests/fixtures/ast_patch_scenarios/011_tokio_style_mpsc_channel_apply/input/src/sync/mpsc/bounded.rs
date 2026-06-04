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
    if buffer == 0 {
        return (Sender { value: None }, Receiver { value: None });
    }
    let sender = Sender { value: None };
    let receiver = Receiver { value: None };
    (sender, receiver)
}
