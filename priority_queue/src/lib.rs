// #![cfg_attr(not(test), no_std)]
#![allow(static_mut_refs)]

use core::cell::Cell;
use core::mem::MaybeUninit;

#[derive(Debug)]
pub struct PriorityQueue<const N: usize, T: Copy + Clone + PartialOrd> {
    data: [(MaybeUninit<T>, Option<u16>); N],
    #[allow(dead_code)]
    prev: Cell<Option<u16>>,
}

struct CsToken;

trait CriticalSection {}

trait PreemptionPoint: CriticalSection {
    fn preemption_point(cs: &CsToken);
}

struct CsSingleCore;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    QueueFull,
}

impl CriticalSection for CsSingleCore {}
impl PreemptionPoint for CsSingleCore {
    #[inline(always)]
    fn preemption_point(_cs: &CsToken) {
        // no-op
    }
}
impl<const N: usize, T: Copy + Clone + PartialOrd> PriorityQueue<N, T> {
    #[inline(always)]
    const fn head(&self) -> Option<u16> {
        self.data[0].1
    }

    #[inline(always)]
    const fn set_head(&mut self, head: Option<u16>) {
        self.data[0].1 = head;
    }

    #[inline(always)]
    const fn free(&self) -> Option<u16> {
        self.data[1].1
    }

    #[inline(always)]
    const fn set_free(&mut self, free: Option<u16>) {
        self.data[1].1 = free;
    }

    #[allow(clippy::new_without_default)]
    #[inline(always)]
    pub const fn new() -> Self {
        let mut pq = Self {
            data: [(MaybeUninit::uninit(), None); N],
            prev: Cell::new(None),
        };

        // initialize head
        pq.set_head(None);

        // initialize free list
        pq.set_free(Some(2));

        let mut i = 2;

        while i < N {
            pq.data[i].1 = if i < N - 1 {
                Some((i + 1) as u16)
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
            let (value, next) = self.data[i as usize];
            self.set_head(next); // update head 
            println!("next {:?}", self.head());
            self.data[i as usize].1 = self.free(); // update free list

            self.set_free(Some(i));
            println!("free {:?}", self.free());

            Some(unsafe { value.assume_init() })
        } else {
            None
        }
    }

    #[inline(always)]
    unsafe fn peek_at(&self, index: u16) -> T {
        unsafe { self.data[index as usize].0.assume_init() }
    }

    #[allow(clippy::manual_map)]
    #[inline(always)]
    pub fn peek(&self) -> Option<T> {
        if let Some(i) = self.head() {
            Some(unsafe { self.peek_at(i) })
        } else {
            None
        }
    }

    #[inline(always)]
    fn insert_first(&mut self, value: T, free_index: u16, next: Option<u16>) {
        self.set_free(self.data[free_index as usize].1); // allocated new node from free list
        self.data[free_index as usize] = (MaybeUninit::new(value), next); // Last node
        self.set_head(Some(free_index)); // Update head to new node
    }

    #[inline(always)]
    fn insert_at(
        &mut self,
        value: T,
        prev_index: u16,
        free_index: u16,
        next: Option<u16>,
    ) -> Result<(), Error> {
        self.set_free(self.data[free_index as usize].1); // allocated new node from free list
        self.data[free_index as usize] = (MaybeUninit::new(value), next); // Last node
        self.data[prev_index as usize].1 = Some(free_index); // Update previous node to new node
        Ok(())
    }

    #[allow(clippy::result_unit_err)]
    #[inline(always)]
    pub fn insert(&mut self, value: T) -> Result<(), Error> {
        // check if free list is not empty
        if let Some(free_index) = self.free() {
            // check if list is not empty
            if let Some(head_index) = self.head() {
                // list is not empty, find correct position to insert
                if value < self.peek().unwrap() {
                    // less then first element
                    self.insert_first(value, free_index, Some(head_index));
                    #[allow(clippy::needless_return)]
                    return Ok(());
                } else {
                    // find the correct position to insert
                    let mut prev_index = head_index;

                    // mock
                    let cs = CsToken;
                    loop {
                        // check if last node
                        match self.data[prev_index as usize].1 {
                            None => {
                                // we reached the end of the list, insert at the end
                                return self.insert_at(value, prev_index, free_index, None);
                            }
                            Some(next_index) => {
                                // smaller than next node,
                                if value < unsafe { self.peek_at(next_index) } {
                                    return self.insert_at(
                                        value,
                                        prev_index,
                                        free_index,
                                        Some(next_index),
                                    );
                                } else {
                                    // move to next node
                                    prev_index = next_index;
                                }
                            }
                        }
                        CsSingleCore::preemption_point(&cs);
                    }
                }
            } else {
                // list is empty, insert first node
                self.insert_first(value, free_index, None);
                #[allow(clippy::needless_return)]
                return Ok(());
            }
        } else {
            Err(Error::QueueFull)
        }
    }
}

unsafe impl<const S: usize, T: Copy + Clone + PartialOrd> Send for PriorityQueue<S, T> {}
unsafe impl<const S: usize, T: Copy + Clone + PartialOrd> Sync for PriorityQueue<S, T> {}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let pq = PriorityQueue::<5, i32>::new();
        println!("{:?}", pq);
        assert_eq!(pq.data[0].1, None);
        assert_eq!(pq.data[1].1, Some(2));
    }

    #[test]
    fn test_pop() {
        let mut pq = PriorityQueue::<5, i32> {
            data: [
                (MaybeUninit::uninit(), Some(2)),
                (MaybeUninit::uninit(), None),
                (MaybeUninit::new(1), Some(3)),
                (MaybeUninit::new(2), Some(4)),
                (MaybeUninit::new(3), None),
            ],
            prev: Cell::new(None),
        };

        println!("{:?}", pq);
        assert_eq!(pq.pop(), Some(1));
        println!("{:?}", pq);
        assert_eq!(pq.pop(), Some(2));
        println!("{:?}", pq);
        assert_eq!(pq.pop(), Some(3));
        println!("{:?}", pq);
        assert_eq!(pq.head(), None);
        assert_eq!(pq.free(), Some(4));
    }

    #[test]
    fn test_insert_first() {
        unsafe {
            static mut PQ: PriorityQueue<5, i32> = PriorityQueue::<5, i32>::new();
            println!("{:?}", PQ);
            assert_eq!(PQ.head(), None);
            assert_eq!(PQ.free(), Some(2));
            assert_eq!(PQ.peek(), None);

            assert_eq!(PQ.insert(3), Ok(()));
            println!("{:?}", PQ);
            assert_eq!(PQ.peek(), Some(3));
            assert_eq!(PQ.head(), Some(2));
            assert_eq!(PQ.free(), Some(3));

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
            assert_eq!(PQ.free(), Some(2));
            assert_eq!(PQ.pop(), None);
        }
    }

    #[test]
    fn test_insert_middle() {
        let mut pq = PriorityQueue::<5, i32>::new();
        println!("{:?}", pq);

        assert_eq!(pq.insert(2), Ok(()));
        println!("{:?}", pq);
        assert_eq!(pq.peek(), Some(2));
        assert_eq!(pq.head(), Some(2));
        assert_eq!(pq.free(), Some(3));

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
