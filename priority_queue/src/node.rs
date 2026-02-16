use core::mem::MaybeUninit;
use core::num::NonZeroU16;

pub(crate) type NodePtr = Option<NonZeroU16>;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Node<T: Copy> {
    pub data: MaybeUninit<T>,
    pub next: NodePtr,
    // Reference counter. A bitfield with bit 15 denotes whether the node has been marked for
    // deletion.
    pub rc: u16,
}

impl<T: Copy> Node<T> {
    #[inline]
    pub const fn new(data: T, ptr: NodePtr) -> Self {
        Self {
            data: MaybeUninit::new(data),
            next: ptr,
            rc: 0,
        }
    }

    #[cfg(test)]
    #[inline]
    pub const fn new_uninit(ptr: NodePtr) -> Self {
        Self {
            data: MaybeUninit::uninit(),
            next: ptr,
            rc: 0,
        }
    }

    #[inline]
    pub const fn new_empty() -> Self {
        Self {
            data: MaybeUninit::uninit(),
            next: None,
            rc: 0,
        }
    }

    #[inline]
    pub fn mark_for_deletion(&mut self) {
        self.rc |= 1 << 15;
    }

    #[inline]
    pub fn mark_ready(&mut self) {
        self.rc &= !(1 << 15);
    }

    #[inline]
    pub fn is_marked_for_deletion(&self) -> bool {
        self.rc & 1 << 15 != 0
    }

    pub fn inc_readers(&mut self) {
        self.rc += 1;
    }

    pub fn dec_readers(&mut self) {
        self.rc = (self.rc & 1 << 15) | (self.rc & 0x7ff).saturating_sub(1);
    }

    pub fn is_being_read(&self) -> bool {
        self.rc & 1 << 15 != 0
    }
}

impl<T: Copy> Default for Node<T> {
    #[inline]
    fn default() -> Self {
        Self::new_empty()
    }
}
