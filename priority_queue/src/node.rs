use core::mem::MaybeUninit;
use core::num::NonZeroU16;

pub(crate) type NodePtr = Option<NonZeroU16>;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Node<T: Copy> {
    pub data: MaybeUninit<T>,
    pub next: NodePtr,
}

impl<T: Copy> Node<T> {
    #[inline]
    pub const fn new(data: T, ptr: NodePtr) -> Self {
        Self {
            data: MaybeUninit::new(data),
            next: ptr,
        }
    }

    #[cfg(test)]
    #[inline]
    pub const fn new_uninit(ptr: NodePtr) -> Self {
        Self {
            data: MaybeUninit::uninit(),
            next: ptr,
        }
    }

    #[inline]
    pub const fn new_empty() -> Self {
        Self {
            data: MaybeUninit::uninit(),
            next: None,
        }
    }
}

impl<T: Clone + Copy + PartialOrd> Default for Node<T> {
    #[inline]
    fn default() -> Self {
        Self::new_empty()
    }
}
