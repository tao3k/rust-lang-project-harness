# Async Sync Lock Boundary

The synchronous critical section ends before the async suspension point. If the
guard must live across awaits, move the state boundary to `tokio::sync`.
