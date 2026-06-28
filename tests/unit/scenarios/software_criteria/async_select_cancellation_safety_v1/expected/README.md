# Expected Shape

The exact read owns the partial-progress buffer outside `tokio::select!`, so
dropping a select branch cannot discard an in-flight `read_exact` operation.
