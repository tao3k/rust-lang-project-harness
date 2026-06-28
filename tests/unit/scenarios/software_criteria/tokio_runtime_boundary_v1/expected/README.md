# Tokio Runtime Boundary

The expected fixture keeps Tokio runtime operations in one facade so task
tracking, shutdown, cancellation, tracing, and runtime thread model choices have
one owner.
