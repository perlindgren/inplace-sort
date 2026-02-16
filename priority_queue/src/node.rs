use core::mem::MaybeUninit;
use core::num::NonZeroU16;

pub(crate) type NodePtr = Option<NonZeroU16>;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Node<T: Clone + Copy> {
    pub data: MaybeUninit<T>,
    pub ptr: NodePtr,
}

impl<T: Clone + Copy> Node<T> {
    #[inline]
    pub const fn new(data: T, ptr: NodePtr) -> Self {
        Self {
            data: MaybeUninit::new(data),
            ptr,
        }
    }

    #[cfg(test)]
    #[inline]
    pub const fn new_uninit(ptr: NodePtr) -> Self {
        Self {
            data: MaybeUninit::uninit(),
            ptr,
        }
    }

    #[inline]
    pub const fn new_empty() -> Self {
        Self {
            data: MaybeUninit::uninit(),
            ptr: None,
        }
    }
}

impl<T: Clone + Copy + PartialOrd> Default for Node<T> {
    #[inline]
    fn default() -> Self {
        Self::new_empty()
    }
}
