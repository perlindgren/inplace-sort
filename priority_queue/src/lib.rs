// #![cfg_attr(not(test), no_std)]
#![allow(static_mut_refs)]

use core::mem::MaybeUninit;

use std::fmt;
use std::fmt::Debug;

#[derive(Debug, Copy, Clone)]
pub struct Cursor<T> {
    min_index: Option<u16>, // None indicates that index refers to head
    min_value: T,
    current_index: u16,
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
        println!("-- critical section start --");
        let result = f(CsToken);
        println!("-- critical section end --");
        result
    }
}

trait Preemption: CriticalSection {
    fn preemption_point(_cs: &CsToken) {
        println!("-- preemption point --");
    }

    fn preemption_region<R>(cs: CsToken, f: impl FnOnce() -> R) -> (CsToken, R) {
        // no-op
        println!("-- preemption region start --");
        let result = f();
        println!("-- preemption region end --");
        (cs, result)
    }
}

struct CsSingleCore;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    QueueFull,
}

impl CriticalSection for CsSingleCore {}
impl Preemption for CsSingleCore {}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MockTest {
    None,
    Insert(i32),
    Pop,
}

impl<const N: usize> PriorityQueue<N, i32> {
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
    pub fn extractMin(&mut self, mock_test: MockTest) -> Option<i32> {
        CsSingleCore::with(|mut cs| {
            let head_index = self.head?;

            let mut current_index = {
                if let Some(cursor) = self.cursor {
                    println!(
                        "extractMin: restore cursor at index {:?}, value {:?}",
                        cursor.current_index, cursor.min_value
                    );
                    cursor.current_index
                } else {
                    println!("extractMin: initialize cursor at head index {}", head_index);
                    self.cursor = Some(Cursor {
                        min_value: unsafe { self.data[head_index as usize].assume_init() },
                        min_index: None,
                        current_index: head_index,
                    });
                    head_index
                }
            };

            println!("extractMin: cursor {:?}", self.cursor);

            while let Some(next_index) = self.next[current_index as usize] {
                let next_value = unsafe { self.data[next_index as usize].assume_init() };
                println!(
                    "extractMin: -- cursor {:?},  current_index {}, next_index {}, next_value {:?}",
                    self.cursor, current_index, next_index, next_value
                );

                if next_value < self.cursor.unwrap().min_value {
                    println!(
                        "update cursor to next_index {}, next_value {:?}",
                        next_index, next_value
                    );
                    self.cursor = Some(Cursor {
                        min_value: next_value,
                        min_index: Some(current_index),
                        current_index: next_index,
                    });
                }

                current_index = next_index;

                // CsSingleCore::preemption_point(&_cs);
                (cs, _) = CsSingleCore::preemption_region(cs, || {
                    println!("-- preemption section in extractMin --");
                    match mock_test {
                        MockTest::None => {}
                        MockTest::Insert(value) => {
                            println!("-------------- mock insert {} in preemption section", value);
                            let _ = self.insert(value);
                        }
                        MockTest::Pop => {
                            println!("-------------- mock pop in preemption section");
                            let val = self.extractMin(MockTest::None);
                            println!(
                                "-------------- mock pop extracted value {:?} in preemption section",
                                val
                            );
                            assert!(
                                self.cursor.is_none(),
                                "cursor should be None after pop in preemption section"
                            );
                        }
                    }
                });

                // restore state from cursor
                if self.cursor.is_none() {
                    break;
                }
            }

            if let Some(cursor) = self.cursor {
                println!("extract at cursor {:?}", cursor);

                if let Some(current) = cursor.min_index {
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
                        println!("update tail to cursor index {:?}", cursor.min_index);
                        self.tail = cursor.min_index;
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
                        println!("update tail to cursor index {:?}", cursor.min_index);
                        self.tail = cursor.min_index;
                    }
                }
                self.cursor = None;
                Some(cursor.min_value)
            } else {
                None
            }
        })
    }

    #[inline(always)]
    fn insert(&mut self, value: i32) -> Result<(), Error> {
        CsSingleCore::with(|_cs| {
            let new_index = self.free.ok_or(Error::QueueFull)?;

            self.data[new_index as usize] = MaybeUninit::new(value);
            self.free = self.next[new_index as usize];
            self.next[new_index as usize] = None; // new node points to None
            if let Some(tail_index) = self.tail {
                self.next[tail_index as usize] = Some(new_index); // old tail points to new node
            } else {
                self.head = Some(new_index);
            }
            self.tail = Some(new_index); // if the queue was empty, set tail to new node

            Ok(())
        })
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

        println!("after insert42 {}", pq);

        println!("extractMin first time");
        assert_eq!(pq.extractMin(MockTest::None), Some(42));
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
        assert_eq!(pq.extractMin(MockTest::None), Some(38));
        println!("after extractMin: {}", pq);

        println!("extractMin second time");
        assert_eq!(pq.extractMin(MockTest::None), Some(42));
        println!("after extractMin: {}", pq);
    }

    #[test]
    fn test_extract_min_42_38_pop() {
        let mut pq = PriorityQueue::<3, i32>::new();
        println!("after init: {}", pq);
        println!("insert 42, 38");
        let _ = pq.insert(42);
        let _ = pq.insert(38);

        println!("42_38{}", pq);

        println!("extractMin first time");
        assert_eq!(pq.extractMin(MockTest::Pop), None);
        println!("after extractMin: {}", pq);

        println!("extractMin second time");
        assert_eq!(pq.extractMin(MockTest::None), Some(42));
        println!("after extractMin: {}", pq);
    }

    #[test]
    fn test_extract_min_42_38_insert() {
        let mut pq = PriorityQueue::<3, i32>::new();
        println!("after init: {}", pq);
        println!("insert 42, 38");
        let _ = pq.insert(42);
        let _ = pq.insert(38);

        println!("42_38{}", pq);

        println!("extractMin first time");
        assert_eq!(pq.extractMin(MockTest::Insert(1337)), Some(38));
        println!("after extractMin: {}", pq);

        println!("extractMin second time");
        assert_eq!(pq.extractMin(MockTest::None), Some(42));
        println!("after extractMin: {}", pq);

        println!("extractMin second time");
        assert_eq!(pq.extractMin(MockTest::None), Some(1337));
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
        assert_eq!(pq.extractMin(MockTest::None), Some(38));
        println!("after extractMin: {}", pq);

        println!("extractMin second time");
        assert_eq!(pq.extractMin(MockTest::None), Some(42));
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
        assert_eq!(pq.extractMin(MockTest::None), Some(38));
        println!("after extractMin: {}", pq);

        println!("extractMin second time");
        assert_eq!(pq.extractMin(MockTest::None), Some(42));
        println!("after extractMin: {}", pq);

        println!("extractMin third time");
        assert_eq!(pq.extractMin(MockTest::None), Some(1337));
        println!("after extractMin: {}", pq);
    }

    #[test]
    fn test_extract_min_large() {
        let mut pq = PriorityQueue::<9, i32>::new();
        println!("after init: {}", pq);

        let _ = pq.insert(38);
        let _ = pq.insert(42);
        let _ = pq.insert(1337);
        let _ = pq.insert(38);
        let _ = pq.insert(42);
        let _ = pq.insert(1337);
        let _ = pq.insert(38);
        let _ = pq.insert(42);
        let _ = pq.insert(1337);

        assert_eq!(pq.extractMin(MockTest::None), Some(38));
        assert_eq!(pq.extractMin(MockTest::None), Some(38));
        assert_eq!(pq.extractMin(MockTest::None), Some(38));
        assert_eq!(pq.extractMin(MockTest::None), Some(42));
        assert_eq!(pq.extractMin(MockTest::None), Some(42));
        assert_eq!(pq.extractMin(MockTest::None), Some(42));
        assert_eq!(pq.extractMin(MockTest::None), Some(1337));
        assert_eq!(pq.extractMin(MockTest::None), Some(1337));
        assert_eq!(pq.extractMin(MockTest::None), Some(1337));
        println!("after extractMin: {}", pq);
    }

    #[test]
    fn test_cs_ps() {
        CsSingleCore::with(|_cs| {
            println!("in critical section");
            CsSingleCore::preemption_point(&_cs);
            CsSingleCore::preemption_region(_cs, || {
                println!("in preemption section");
            });
        });
    }
}
