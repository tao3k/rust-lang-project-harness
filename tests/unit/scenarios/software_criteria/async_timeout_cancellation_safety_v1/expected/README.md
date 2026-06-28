# Expected Shape

The timed boundary no longer owns exact I/O partial progress; exact reads happen outside the future that `tokio::time::timeout` may drop.
