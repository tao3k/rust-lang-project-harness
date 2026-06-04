pub mod scheduler;

pub struct Handle;

impl Clone for Handle {
    fn clone(&self) -> Self {
        Self
    }
}
