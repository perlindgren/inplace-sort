pub struct CsToken;

pub trait CriticalSection {}

pub trait PreemptionPoint: CriticalSection {
    fn preemption_point(cs: &CsToken);
}

pub struct CsSingleCore;

impl CriticalSection for CsSingleCore {}
impl PreemptionPoint for CsSingleCore {
    #[inline(always)]
    fn preemption_point(_cs: &CsToken) {
        // no-op
    }
}
