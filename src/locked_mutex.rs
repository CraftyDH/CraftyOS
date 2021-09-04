use spin::{Mutex, MutexGuard};

// A trait that locks an arbitrary item behind a spin mutex
pub struct Locked<A> {
    inner: Mutex<A>
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Self {
            inner: Mutex::new(inner)
        }
    }

    pub fn lock(&self) -> MutexGuard<A> {
        self.inner.lock()
    }
}