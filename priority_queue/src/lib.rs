// #![cfg_attr(not(test), no_std)]
#![allow(static_mut_refs)]
use core::mem::MaybeUninit;
use std::num::NonZeroU16;

mod mock_cs;
use mock_cs::{CsSingleCore, CsToken, PreemptionPoint};

// Helper trait
trait AsUsize {
    fn to_usize(self) -> usize;
}

impl AsUsize for NonZeroU16 {
    #[inline(always)]
    fn to_usize(self) -> usize {
        self.get() as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    QueueFull,
}

#[derive(Debug, Clone, Copy)]
pub struct PriorityQueue<T, const N: usize>
where
    T: Copy + Clone + PartialOrd,
{
    data: [(MaybeUninit<T>, Option<NonZeroU16>); N],
    // head: Option<u16>,
    // free: Option<u16>,
}

impl<T, const N: usize> Default for PriorityQueue<T, N>
where
    T: Copy + Clone + PartialOrd,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> PriorityQueue<T, N>
where
    T: Copy + Clone + PartialOrd,
{
    #[inline(always)]
    const fn head(&self) -> Option<NonZeroU16> {
        self.data[0].1
    }

    #[inline(always)]
    const fn set_head(&mut self, head: Option<NonZeroU16>) {
        self.data[0].1 = head;
    }

    #[inline(always)]
    const fn free(&self) -> Option<NonZeroU16> {
        self.data[1].1
    }

    #[inline(always)]
    const fn set_free(&mut self, free: Option<NonZeroU16>) {
        self.data[1].1 = free;
    }

    #[inline(always)]
    pub const fn new() -> Self {
        let mut pq = Self {
            data: [(MaybeUninit::uninit(), None); N],
        };

        // initialize head
        pq.set_head(None);

        // initialize free list
        pq.set_free(NonZeroU16::new(2));

        // It's stupid we can't use for loops in const fns :(
        let mut i = 2;
        while i < N {
            pq.data[i].1 = if i < N - 1 {
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
        if let Some(i) = self.head() {
            let (value, next) = self.data[i.to_usize()];
            self.set_head(next); // update head 
            println!("next {:?}", self.head());
            self.data[i.to_usize()].1 = self.free(); // update free list

            self.set_free(Some(i));
            println!("free {:?}", self.free());

            Some(unsafe { value.assume_init() })
        } else {
            None
        }
    }

    #[inline(always)]
    unsafe fn peek_at(&self, index: NonZeroU16) -> T {
        unsafe { self.data[index.to_usize()].0.assume_init() }
    }

    #[inline(always)]
    pub fn peek(&self) -> Option<T> {
        self.head().map(|i| unsafe { self.peek_at(i) })
    }

    #[inline(always)]
    fn insert_first(&mut self, value: T, free_index: NonZeroU16, next: Option<NonZeroU16>) {
        self.set_free(self.data[free_index.to_usize()].1); // allocated new node from free list
        self.data[free_index.to_usize()] = (MaybeUninit::new(value), next); // Last node
        self.set_head(Some(free_index)); // Update head to new node
    }

    #[inline(always)]
    fn insert_at(
        &mut self,
        value: T,
        prev_index: NonZeroU16,
        free_index: NonZeroU16,
        next: Option<NonZeroU16>,
    ) -> Result<(), Error> {
        self.set_free(self.data[free_index.to_usize()].1); // allocated new node from free list
        self.data[free_index.to_usize()] = (MaybeUninit::new(value), next); // Last node
        self.data[prev_index.to_usize()].1 = Some(free_index); // Update previous node to new node
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
                match self.data[prev_index.to_usize()].1 {
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
                CsSingleCore::preemption_point(&cs);
            }
        }
    }
}

unsafe impl<T: Copy + Clone + PartialOrd, const N: usize> Send for PriorityQueue<T, N> {}
unsafe impl<T: Copy + Clone + PartialOrd, const N: usize> Sync for PriorityQueue<T, N> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let pq = PriorityQueue::<i32, 5>::new();
        println!("{:?}", pq);
        assert_eq!(pq.data[0].1, None);
        assert_eq!(pq.data[1].1, NonZeroU16::new(2));
    }

    #[test]
    fn test_pop() {
        let mut pq = PriorityQueue::<i32, 5> {
            data: [
                (MaybeUninit::uninit(), NonZeroU16::new(2)),
                (MaybeUninit::uninit(), None),
                (MaybeUninit::new(1), NonZeroU16::new(3)),
                (MaybeUninit::new(2), NonZeroU16::new(4)),
                (MaybeUninit::new(3), None),
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
