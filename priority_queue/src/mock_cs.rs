pub struct CsToken<T> {
    _inner: T,
}

impl<T> CsToken<T> {
    pub const unsafe fn new(_inner: T) -> Self {
        Self { _inner }
    }
}

pub trait CriticalSection {
    type Inner;
    fn enter(&self) -> CsToken<Self::Inner>;
}

pub trait PreemptionPoint: CriticalSection {
    fn preemption_point(cs: &mut CsToken<Self::Inner>);
}

#[derive(Debug)]
pub struct NoopCs;

impl CriticalSection for NoopCs {
    type Inner = ();
    fn enter(&self) -> CsToken<Self::Inner> {
        // noop
        CsToken { _inner: () }
    }
}
impl PreemptionPoint for NoopCs {
    #[inline(always)]
    fn preemption_point(_cs: &mut CsToken<Self::Inner>) {
        // no-op
    }
}

#[cfg(test)]
pub use mutex_cs::*;

#[cfg(test)]
mod mutex_cs {
    use std::{
        sync::{Mutex, MutexGuard},
        time::Duration,
    };

    use super::*;

    static MUTEX: Mutex<()> = Mutex::new(());

    pub struct MutexCs {
        wait_time: Duration,
    }

    impl MutexCs {
        pub fn new(wait_time: Duration) -> Self {
            Self { wait_time }
        }
    }

    impl CriticalSection for MutexCs {
        type Inner = Option<(MutexGuard<'static, ()>, Duration)>;

        fn enter(&self) -> CsToken<Self::Inner> {
            CsToken {
                _inner: Some((MUTEX.lock().unwrap(), self.wait_time)),
            }
        }
    }

    impl PreemptionPoint for MutexCs {
        fn preemption_point(cs: &mut CsToken<Self::Inner>) {
            let (guard, wait_time) = cs._inner.take().expect("Some bug happened");
            drop(guard);

            // Sleep to simulate preemption
            std::thread::sleep(Duration::from_millis(500));

            cs._inner.replace((MUTEX.lock().unwrap(), wait_time));
        }
    }
}
