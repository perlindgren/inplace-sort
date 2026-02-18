use core::cell::UnsafeCell;

use critical_section::CriticalSection;

#[derive(Debug)]
pub struct Mutex<T> {
    inner: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    /// Creates a new mutex.
    #[inline]
    pub const fn new(value: T) -> Self {
        Mutex {
            inner: UnsafeCell::new(value),
        }
    }

    /// Gets a mutable reference to the contained value when the mutex is
    /// already uniquely borrowed.
    ///
    /// This does not require locking or a critical section since it takes `&mut
    /// self`, which guarantees unique ownership already. Care must be taken
    /// when using this method to **unsafely** access `static mut`
    /// variables, appropriate fences must be used to prevent
    /// unwanted optimizations.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.inner.get() }
    }

    /// Unwraps the contained value, consuming the mutex.
    #[inline]
    pub fn into_inner(self) -> T {
        self.inner.into_inner()
    }

    /// Borrows the data.
    ///
    /// # Safety:
    ///
    /// As opposed to [`critical_section::Mutex`], this Mutex provides interior
    /// mutability. Since we are aiming for pure performance, we want to avoid
    /// runtime checks via [`RefCell`](core::cell::RefCell) or similar.
    /// Therefore, there are certain invariants which must be manually
    /// respected:
    /// * This method may not be used with a [`CriticalSection`] created from
    ///   nested critical sections.
    #[inline]
    pub fn borrow<'cs>(&self, _cs: CriticalSection<'cs>) -> *mut T {
        self.inner.get()
    }

    pub const unsafe fn borrow_unsafe(&self) -> *mut T {
        self.inner.get()
    }
}
