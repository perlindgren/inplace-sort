use core::cell::UnsafeCell;

#[derive(Debug)]
pub struct Mutex<T> {
    // The `UnsafeCell` is not strictly necessary here: In theory, just using `T` should
    // be fine.
    // However, without `UnsafeCell`, the compiler may use niches inside `T`, and may
    // read the niche value _without locking the mutex_. As we don't provide interior
    // mutability, this is still not violating any aliasing rules and should be perfectly
    // fine. But as the cost of adding `UnsafeCell` is very small, we add it out of
    // cautiousness, just in case the reason `T` is not `Sync` in the first place is
    // something very obscure we didn't consider.
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

    /// Gets a mutable reference to the contained value when the mutex is already uniquely borrowed.
    ///
    /// This does not require locking or a critical section since it takes `&mut self`, which
    /// guarantees unique ownership already. Care must be taken when using this method to
    /// **unsafely** access `static mut` variables, appropriate fences must be used to prevent
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

    /// Borrows the data for the duration of the critical section.
    #[inline]
    pub fn borrow<'cs>(&'cs self, _cs: critical_section::CriticalSection<'cs>) -> &'cs mut T {
        unsafe { &mut *self.inner.get() }
    }
}
