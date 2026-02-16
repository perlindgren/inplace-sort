#![cfg_attr(not(feature = "std"), no_std)]
#![allow(static_mut_refs)]

use core::num::NonZeroU16;

pub(crate) mod mock_cs;
use mock_cs::{CsSingleCore, CsToken, PreemptionPoint};

pub(crate) mod node;
use node::*;

/// Helper trait to convert a [`NonZeroU16`] into a [`usize`]
trait ToUsize {
    fn to_usize(self) -> usize;
}

impl ToUsize for NonZeroU16 {
    #[inline(always)]
    fn to_usize(self) -> usize {
        self.get() as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    QueueFull,
}

#[derive(Debug)]
pub struct PriorityQueue<T, const N: usize>
where
    T: Clone + Copy,
{
    data: [Node<T>; N],
    // head: Option<u16>,
    // free: Option<u16>,
}

impl<T, const N: usize> Default for PriorityQueue<T, N>
where
    T: PartialOrd + Clone + Copy,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> PriorityQueue<T, N>
where
    T: PartialOrd + Clone + Copy,
{
    #[inline(always)]
    const fn head(&self) -> NodePtr {
        self.data[0].ptr
    }

    #[inline(always)]
    const fn set_head(&mut self, head: NodePtr) {
        self.data[0].ptr = head;
    }

    #[inline(always)]
    const fn free(&self) -> NodePtr {
        self.data[1].ptr
    }

    #[inline(always)]
    const fn set_free(&mut self, free: NodePtr) {
        self.data[1].ptr = free;
    }

    #[inline(always)]
    pub const fn new() -> Self {
        let mut pq = Self {
            data: [const { Node::new_empty() }; N],
        };

        // initialize head
        pq.set_head(None);

        // initialize free list
        pq.set_free(NonZeroU16::new(2));

        // It's stupid we can't use for loops in const fns :(
        let mut i = 2;
        while i < N {
            pq.data[i].ptr = if i < N - 1 {
                NonZeroU16::new(i as u16 + 1)
            } else {
                None
            };
            i += 1;
        }

        pq
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        // Return None directly if list is empty
        let i = self.head()?;

        // SAFETY: no need to check, we already verified list isn't empty
        let node = self.data[i.to_usize()];
        let next = node.ptr;

        // Update head
        self.set_head(next);

        #[cfg(feature = "std")]
        println!("next {:?}", self.head());

        // Update free list
        self.data[i.to_usize()].ptr = self.free();
        self.set_free(Some(i));

        #[cfg(feature = "std")]
        println!("free {:?}", self.free());

        Some(unsafe { node.data.assume_init() })
    }

    #[inline(always)]
    unsafe fn peek_at(&self, index: NonZeroU16) -> T {
        unsafe { self.data[index.to_usize()].data.assume_init() }
    }

    // TODO: this should be lock-protected?
    #[inline(always)]
    pub fn peek(&self) -> Option<T> {
        self.head().map(|i| unsafe { self.peek_at(i) })
    }

    #[inline(always)]
    fn insert_first(&mut self, value: T, free_index: NonZeroU16, next: NodePtr) {
        // Allocated new node from free list
        self.set_free(self.data[free_index.to_usize()].ptr);

        // Last node
        self.data[free_index.to_usize()] = Node::new(value, next);

        // Update head to new node
        self.set_head(Some(free_index));
    }

    #[inline(always)]
    fn insert_at(
        &mut self,
        value: T,
        prev_index: NonZeroU16,
        free_index: NonZeroU16,
        next: NodePtr,
    ) -> Result<(), Error> {
        // Allocated new node from free list
        self.set_free(self.data[free_index.to_usize()].ptr);

        // Last node
        self.data[free_index.to_usize()] = Node::new(value, next);

        // Update previous node to new node
        self.data[prev_index.to_usize()].ptr = Some(free_index);

        Ok(())
    }

    #[inline(always)]
    pub fn insert(&mut self, value: T) -> Result<(), Error> {
        // check if free list is not empty
        let Some(free_index) = self.free() else {
            return Err(Error::QueueFull);
        };

        // check if list is not empty
        let Some(head_index) = self.head() else {
            // list is empty, insert first node
            self.insert_first(value, free_index, None);
            return Ok(());
        };

        // List is not empty, find correct position to insert
        // TODO: can unwrap_unchecked here? We've confirmed that list isn't empty
        if value < self.peek().unwrap() {
            // less then first element
            self.insert_first(value, free_index, Some(head_index));
            Ok(())
        } else {
            // find the correct position to insert
            let mut prev_index = head_index;

            // mock
            let cs = CsToken;
            loop {
                // check if last node
                match self.data[prev_index.to_usize()].ptr {
                    None => {
                        // we reached the end of the list, insert at the end
                        return self.insert_at(value, prev_index, free_index, None);
                    }

                    Some(next_index) => {
                        if value < unsafe { self.peek_at(next_index) } {
                            // smaller than next node,
                            return self.insert_at(value, prev_index, free_index, Some(next_index));
                        } else {
                            // move to next node
                            prev_index = next_index;
                        }
                    }
                }
                // TODO: what happens if the node at next_index gets popped inside the yield point?
                CsSingleCore::preemption_point(&cs);
            }
        }
    }
}

unsafe impl<T: Copy + Clone, const N: usize> Send for PriorityQueue<T, N> {}
unsafe impl<T: Copy + Clone, const N: usize> Sync for PriorityQueue<T, N> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let pq = PriorityQueue::<i32, 5>::new();
        println!("{:?}", pq);
        assert_eq!(pq.data[0].ptr, None);
        assert_eq!(pq.data[1].ptr, NonZeroU16::new(2));
    }

    #[test]
    fn test_pop() {
        let mut pq = PriorityQueue::<i32, 5> {
            data: [
                // Head
                Node::new_uninit(NonZeroU16::new(2)),
                // Free
                Node::new_uninit(None),
                Node::new(1, NonZeroU16::new(3)),
                Node::new(2, NonZeroU16::new(4)),
                Node::new(3, None),
            ],
            //         head: NonZeroU16::new(0),
            //         free: None,
        };

        println!("{:?}", pq);
        assert_eq!(pq.pop(), Some(1));
        println!("{:?}", pq);
        assert_eq!(pq.pop(), Some(2));
        println!("{:?}", pq);
        assert_eq!(pq.pop(), Some(3));
        println!("{:?}", pq);
        assert_eq!(pq.head(), None);
        assert_eq!(pq.free(), NonZeroU16::new(4));
    }

    #[test]
    fn test_insert_first() {
        unsafe {
            static mut PQ: PriorityQueue<i32, 5> = PriorityQueue::<i32, 5>::new();
            println!("{:?}", PQ);
            assert_eq!(PQ.head(), None);
            assert_eq!(PQ.free(), NonZeroU16::new(2));
            assert_eq!(PQ.peek(), None);

            assert_eq!(PQ.insert(3), Ok(()));
            println!("{:?}", PQ);
            assert_eq!(PQ.peek(), Some(3));
            assert_eq!(PQ.head(), NonZeroU16::new(2));
            assert_eq!(PQ.free(), NonZeroU16::new(3));

            assert_eq!(PQ.insert(2), Ok(()));
            println!("{:?}", PQ);
            assert_eq!(PQ.peek(), Some(2));
            assert_eq!(PQ.insert(1), Ok(()));
            println!("{:?}", PQ);
            assert_eq!(PQ.peek(), Some(1));
            assert_eq!(PQ.insert(0), Err(Error::QueueFull));
            println!("{:?}", PQ);

            assert_eq!(PQ.pop(), Some(1));
            println!("{:?}", PQ);
            assert_eq!(PQ.pop(), Some(2));
            println!("{:?}", PQ);
            assert_eq!(PQ.pop(), Some(3));
            println!("{:?}", PQ);
            assert_eq!(PQ.head(), None);
            assert_eq!(PQ.free(), NonZeroU16::new(2));
            assert_eq!(PQ.pop(), None);
        }
    }

    #[test]
    fn test_insert_middle() {
        let mut pq = PriorityQueue::<i32, 5>::new();
        println!("{:?}", pq);

        assert_eq!(pq.insert(2), Ok(()));
        println!("{:?}", pq);
        assert_eq!(pq.peek(), Some(2));
        assert_eq!(pq.head(), NonZeroU16::new(2));
        assert_eq!(pq.free(), NonZeroU16::new(3));

        assert_eq!(pq.insert(4), Ok(()));
        println!("{:?}", pq);
        assert_eq!(pq.peek(), Some(2));

        assert_eq!(pq.insert(3), Ok(()));
        println!("{:?}", pq);
        assert_eq!(pq.pop(), Some(2));

        assert_eq!(pq.pop(), Some(3));
        println!("{:?}", pq);

        assert_eq!(pq.pop(), Some(4));
        println!("{:?}", pq);
    }
}
