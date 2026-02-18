use core::mem::MaybeUninit;

pub(crate) type NodePtr = u16;

#[derive(Debug)]
pub(crate) struct Node<T> {
    pub value: MaybeUninit<T>,
    pub next: Option<NodePtr>,
}

impl<T> Node<T> {
    #[inline]
    pub const fn new(data: T, ptr: Option<NodePtr>) -> Self {
        Self {
            value: MaybeUninit::new(data),
            next: ptr,
        }
    }

    #[inline]
    pub const fn new_uninit() -> Self {
        Self {
            value: MaybeUninit::uninit(),
            next: None,
        }
    }

    // #[inline]
    // pub const fn new_empty() -> Self {
    //     Self {
    //         data: MaybeUninit::uninit(),
    //         next: None,
    //     }
    // }
}
