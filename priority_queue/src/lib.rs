// #![cfg_attr(not(test), no_std)]
#![allow(static_mut_refs)]

use core::mem::MaybeUninit;

use std::fmt;
use std::fmt::Debug;

// Reserved queue elements:
// index 0: head index
// index 1: free list head index
// index 2: last node index
#[derive(Debug)]
pub struct PriorityQueue<const N: usize, T: Debug + Copy + Clone + PartialOrd> {
    data: [MaybeUninit<T>; N],
    next: [Option<u16>; N],
    head: Option<u16>,
    tail: Option<u16>,
    free: Option<u16>,
}

impl<const N: usize, T: Debug + Copy + Clone + PartialOrd> fmt::Display for PriorityQueue<N, T> {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "head {:?}, tail {:?}, free {:?}, next {:?}",
            self.head, self.tail, self.free, self.next
        )
    }
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MockTest {
    None,
    Pop,
}

impl<const N: usize, T: Debug + Copy + Clone + PartialOrd> PriorityQueue<N, T> {
    #[allow(clippy::new_without_default)]
    #[inline(always)]
    pub const fn new() -> Self {
        let mut pq = Self {
            data: [MaybeUninit::uninit(); N],
            next: [None; N],
            head: None,
            tail: None,
            free: Some(0),
        };

        let mut i = 0;

        while i < N {
            pq.next[i] = if i < N - 1 {
                Some((i + 1) as u16)
            } else {
                None
            };
            i += 1;
        }

        pq
    }

    #[inline(always)]
    pub fn extractMin(&mut self) -> Option<T> {
        let head_index = self.head?;
        let mut min_value = unsafe { self.data[head_index as usize].assume_init() };
        let mut cursor = 0; // start from the head node
        let mut min_cursor = cursor;

        while let Some(next) = self.next[cursor as usize] {
            let next_value = unsafe { self.data[next as usize].assume_init() };
            println!(
                "cursor {}, value {:?}, next {}, next_value {:?}",
                cursor, min_value, next, next_value
            );

            if next_value < min_value {
                min_value = next_value;
                min_cursor = cursor;
            }
            cursor = next;
        }

        println!("min_value {:?}, min_cursor {}", min_value, min_cursor);
        // let next_of_min_cursor = self.next[self.next[min_cursor as usize].unwrap() as usize];
        // // add to free list
        // self.next[self.next[min_cursor as usize].unwrap() as usize] = self.free();
        // self.set_free(Some(min_cursor));

        //
        // self.next[min_cursor as usize] = next_of_min_cursor;
        // self.set_free(Some(min_cursor));
        // println!(
        //     "EM: min_cursor {}, min_value {:?}, next {:?}",
        //     min_cursor, min_value, next
        // );

        // self.next[min_cursor as usize] = next; // bypass the min node

        Some(min_value)
    }

    #[inline(always)]
    fn insert(&mut self, value: T) -> Result<(), Error> {
        let new_index = self.free.ok_or(Error::QueueFull)?;
        self.data[new_index as usize] = MaybeUninit::new(value);
        self.free = self.next[new_index as usize];
        self.next[new_index as usize] = None; // new node points to None
        self.tail = Some(new_index);
        if self.head.is_none() {
            self.head = Some(new_index);
        }
        Ok(())
    }
}

// unsafe impl<const S: usize, T: Copy + Clone + PartialOrd> Send for PriorityQueue<S, T> {}
// unsafe impl<const S: usize, T: Copy + Clone + PartialOrd> Sync for PriorityQueue<S, T> {}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let pq = PriorityQueue::<3, i32>::new();
        println!("{:?}", pq);
        assert_eq!(pq.head, None);
        assert_eq!(pq.free, Some(0));
    }

    #[test]
    fn test_insert() {
        unsafe {
            static mut PQ: PriorityQueue<3, i32> = PriorityQueue::new();
            println!("{}", PQ);
            assert_eq!(PQ.head, None);
            assert_eq!(PQ.tail, None);
            assert_eq!(PQ.free, Some(0));
            assert_eq!(PQ.insert(45), Ok(()));
            println!("insert 45: {}", PQ);
            assert_eq!(PQ.head, Some(0));
            assert_eq!(PQ.tail, Some(0));
            assert_eq!(PQ.free, Some(1));

            assert_eq!(PQ.insert(12), Ok(()));
            println!("insert 12: {}", PQ);
            assert_eq!(PQ.head, Some(0));
            assert_eq!(PQ.tail, Some(0));
            assert_eq!(PQ.free, Some(1));

            // println!("{:?}", PQ);
            // assert_eq!(PQ.head(), Some(3));
            // assert_eq!(PQ.tail(), Some(4));

            // assert_eq!(PQ.insert(56), Ok(()));
            // println!("{:?}", PQ);
            // assert_eq!(PQ.head(), Some(3));
            // assert_eq!(PQ.tail(), Some(5));
        }
    }

    //#[test]
    // fn test_extract_min() {
    //     let mut pq = PriorityQueue::<6, i32>::new();
    //     println!("after init: {}", pq);
    //     println!("extractMin{:?}", pq.extractMin());
    //     println!("after extractMin: {}", pq);
    //     println!("insert 45");
    //     let _ = pq.insert(45);
    //     println!("{}", pq);

    //     println!("extractMin");
    //     println!("extracted {:?}", pq.extractMin());
    //     // println!("after extractMin: {}", pq);

    //     // assert_eq!(pq.extractMin(), Some(45));

    //     // assert_eq!(pq.extractMin(), None);
    // }
}
