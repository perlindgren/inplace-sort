// #![cfg_attr(not(test), no_std)]
#![allow(static_mut_refs)]

use core::mem::MaybeUninit;

use std::fmt;
use std::fmt::Debug;

#[derive(Debug, Copy, Clone)]
pub struct Cursor<T> {
    index: Option<u16>, // None indicates that index refers to head
    next_value: T,
}
#[derive(Debug)]
pub struct PriorityQueue<const N: usize, T: Debug + Copy + Clone + PartialOrd> {
    data: [MaybeUninit<T>; N],
    next: [Option<u16>; N],
    head: Option<u16>,
    tail: Option<u16>,
    free: Option<u16>,
    cursor: Option<Cursor<T>>, // should be unsafe cell
}

impl<const N: usize, T: Debug + Copy + Clone + PartialOrd> fmt::Display for PriorityQueue<N, T> {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let data = self
            .data
            .iter()
            .map(|e| format!("{:?}", unsafe { e.assume_init() }))
            .collect::<Vec<_>>()
            .join(", ");
        write!(
            f,
            "head {:?}, tail {:?}, free {:?}, next {:?}, data [{}], cursor {:?}",
            self.head, self.tail, self.free, self.next, data, self.cursor
        )
    }
}

struct CsToken;

trait CriticalSection {
    fn with<R>(f: impl FnOnce(CsToken) -> R) -> R {
        // no-op
        f(CsToken)
    }
}

// trait PreemptionPoint: CriticalSection {
//     fn preemption_point(cs: &CsToken);
// }

struct CsSingleCore;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    QueueFull,
}

impl CriticalSection for CsSingleCore {}
// impl PreemptionPoint for CsSingleCore {
//     #[inline(always)]
//     fn preemption_point(_cs: &CsToken) {
//         // no-op
//     }
// }

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
            cursor: None,
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

        if self.cursor.is_none() {
            self.cursor = Some(Cursor {
                next_value: unsafe { self.data[head_index as usize].assume_init() },
                index: None,
            });
        }
        println!("cursor {:?}", self.cursor);
        let mut current_index = head_index;

        while let Some(next_index) = self.next[current_index as usize] {
            let next_value = unsafe { self.data[next_index as usize].assume_init() };
            println!(
                "-- cursor {:?},  current_index {}, next_index {}, next_value {:?}",
                self.cursor, current_index, next_index, next_value
            );

            if next_value < self.cursor.unwrap().next_value {
                println!(
                    "update cursor to next_index {}, next_value {:?}",
                    next_index, next_value
                );
                self.cursor = Some(Cursor {
                    next_value,
                    index: Some(current_index),
                });
            }

            current_index = next_index;
        }

        if let Some(cursor) = self.cursor {
            println!("extract at cursor {:?}", cursor);

            if let Some(current) = cursor.index {
                // extract and free node at current
                let next = self.next[current as usize];
                println!(
                    "current is not head, extract node at current {} with next {:?}",
                    current, next
                );

                // head should not be changed since we have traversed it
                self.next[current as usize] = self.next[next.unwrap() as usize]; // update next of current to skip the extracted node

                // update free list to include the extracted node
                self.next[next.unwrap() as usize] = self.free;
                self.free = next;

                if self.tail == next {
                    println!("update tail to cursor index {:?}", cursor.index);
                    self.tail = cursor.index;
                }
            } else {
                // extract and free last node
                let free_index = self.head.unwrap();
                let next = self.next[free_index as usize];
                println!(
                    "extract last node, free index {}, next {:?}",
                    free_index, next
                );
                self.next[free_index as usize] = self.free; // add to free list
                self.free = Some(free_index); // update free to point to the new free node

                self.head = next; // update head to next node
                if self.tail == Some(free_index) {
                    println!("update tail to cursor index {:?}", cursor.index);
                    self.tail = cursor.index;
                }
            }
            self.cursor = None;
            Some(cursor.next_value)
        } else {
            None
        }
    }

    #[inline(always)]
    fn insert(&mut self, value: T) -> Result<(), Error> {
        let new_index = self.free.ok_or(Error::QueueFull)?;
        let _ = CsSingleCore::with(|_cs: CsToken| {
            self.data[new_index as usize] = MaybeUninit::new(value);
            self.free = self.next[new_index as usize];
            self.next[new_index as usize] = None; // new node points to None
            if let Some(tail_index) = self.tail {
                self.next[tail_index as usize] = Some(new_index); // old tail points to new node
            } else {
                self.head = Some(new_index);
            }
            self.tail = Some(new_index); // if the queue was empty, set tail to new node
        });
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
            assert_eq!(PQ.insert(42), Ok(()));
            println!("insert 42: {}", PQ);
            assert_eq!(PQ.head, Some(0));
            assert_eq!(PQ.tail, Some(0));
            assert_eq!(PQ.free, Some(1));

            assert_eq!(PQ.insert(1337), Ok(()));
            println!("insert 1337: {}", PQ);
            assert_eq!(PQ.head, Some(0));
            assert_eq!(PQ.tail, Some(1));
            assert_eq!(PQ.free, Some(2));

            assert_eq!(PQ.insert(38), Ok(()));
            println!("insert 38: {}", PQ);
            assert_eq!(PQ.head, Some(0));
            assert_eq!(PQ.tail, Some(2));
            assert_eq!(PQ.free, None);

            assert_eq!(PQ.insert(56), Err(Error::QueueFull));
        }
    }

    #[test]
    fn test_extract_min_42() {
        let mut pq = PriorityQueue::<3, i32>::new();
        println!("after init: {}", pq);
        println!("insert 42");
        let _ = pq.insert(42);

        println!("42 {}", pq);

        println!("extractMin first time");
        println!("extracted {:?}", pq.extractMin());
        println!("after extractMin: {}", pq);
    }

    #[test]
    fn test_extract_min_42_38() {
        let mut pq = PriorityQueue::<3, i32>::new();
        println!("after init: {}", pq);
        println!("insert 42, 38");
        let _ = pq.insert(42);
        let _ = pq.insert(38);

        println!("42_38{}", pq);

        println!("extractMin first time");
        println!("extracted {:?}", pq.extractMin());
        println!("after extractMin: {}", pq);

        println!("extractMin second time");
        println!("extracted {:?}", pq.extractMin());
        println!("after extractMin: {}", pq);
    }

    #[test]
    fn test_extract_min_38_42() {
        let mut pq = PriorityQueue::<3, i32>::new();
        println!("after init: {}", pq);
        println!("insert 38, 42");
        let _ = pq.insert(38);
        let _ = pq.insert(42);

        println!("38_42{}", pq);

        println!("extractMin first time");
        println!("extracted {:?}", pq.extractMin());
        println!("after extractMin: {}", pq);

        println!("extractMin second time");
        println!("extracted {:?}", pq.extractMin());
        println!("after extractMin: {}", pq);
    }

    #[test]
    fn test_extract_min_38_42_1337() {
        let mut pq = PriorityQueue::<3, i32>::new();
        println!("after init: {}", pq);
        println!("insert 38, 42, 1337");
        let _ = pq.insert(38);
        let _ = pq.insert(42);
        let _ = pq.insert(1337);

        println!("38_42_1337 {}", pq);

        println!("extractMin first time");

        println!("extracted {:?}", pq.extractMin());
        println!("after extractMin: {}", pq);

        println!("extractMin second time");
        println!("extracted {:?}", pq.extractMin());
        println!("after extractMin: {}", pq);

        println!("extractMin second time");
        println!("extracted {:?}", pq.extractMin());
        println!("after extractMin: {}", pq);
    }
}
