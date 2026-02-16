// #![cfg_attr(not(test), no_std)]
#![allow(static_mut_refs)]

use core::mem::MaybeUninit;

#[derive(Debug)]
pub struct PriorityQueue<const N: usize, T: Copy + Clone + PartialOrd> {
    prev: u16, // index to prev node
    data: [MaybeUninit<T>; N],
    next: [Option<u16>; N],
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

impl<const N: usize, T: Copy + Clone + PartialOrd> PriorityQueue<N, T> {
    #[inline(always)]
    const fn head(&self) -> Option<u16> {
        self.next[0]
    }

    #[inline(always)]
    const fn set_head(&mut self, head: Option<u16>) {
        self.next[0] = head;
    }

    #[inline(always)]
    const fn free(&self) -> Option<u16> {
        self.next[1]
    }

    #[inline(always)]
    const fn set_free(&mut self, free: Option<u16>) {
        self.next[1] = free;
    }

    #[allow(clippy::new_without_default)]
    #[inline(always)]
    pub const fn new() -> Self {
        let mut pq = Self {
            data: [MaybeUninit::uninit(); N],
            next: [None; N],
            prev: 0,
        };

        // initialize head
        pq.set_head(None);

        // initialize free list
        pq.set_free(Some(2));

        let mut i = 2;

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
    pub fn pop(&mut self) -> Option<T> {
        if let Some(i) = self.head() {
            let next = self.next[i as usize];
            self.set_head(next); // update head 
            // println!("next {:?}", self.head());
            self.next[i as usize] = self.free(); // update free list

            self.set_free(Some(i));
            // println!("free {:?}", self.free());

            // if head was the prev node, update prev to next of head
            if self.prev == 0 {
                self.prev = i; // next of head 
            }

            Some(unsafe { self.data[i as usize].assume_init() })
        } else {
            None
        }
    }

    #[inline(always)]
    unsafe fn peek_at(&self, index: u16) -> T {
        unsafe { self.data[index as usize].assume_init() }
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

    // Allocate a new new node from the free list and write value to data, return the index of the new node.
    #[inline(always)]
    fn allocate_node(&mut self, value: T) -> Result<u16, Error> {
        let new_index = self.free().ok_or(Error::QueueFull)?;
        self.data[new_index as usize] = MaybeUninit::new(value);
        self.set_free(self.next[new_index as usize]);
        Ok(new_index)
    }

    #[inline(always)]
    fn insert_at(&mut self, value: T, prev_index: u16, next: Option<u16>) -> Result<(), Error> {
        let new_index = self.allocate_node(value)?; // allocate new node
        self.next[new_index as usize] = next; // New node points to next node
        self.next[prev_index as usize] = Some(new_index); // Previous node points to new node
        Ok(())
    }

    #[allow(clippy::result_unit_err)]
    #[inline(always)]
    pub fn insert(&mut self, value: T, mock: MockTest) -> Result<(), Error> {
        let mut prev_index = 0; // we start from the head

        loop {
            if let Some(next_index) = self.next[prev_index as usize] {
                // smaller than next node,
                if value < unsafe { self.peek_at(next_index) } {
                    return self.insert_at(value, prev_index, Some(next_index));
                } else {
                    self.prev = prev_index; // update prev to current node
                    CsSingleCore::preemption_point(&CsToken);
                    match mock {
                        MockTest::None => {}
                        MockTest::Pop => {
                            println!("-- preempt before pop at index {}", prev_index);

                            self.pop();
                            println!("-- preempt after pop at index {}", prev_index);
                        }
                    }
                    prev_index = self.prev; // read after preemption point 
                }
            } else {
                // we reached the end of the list, insert at the end
                return self.insert_at(value, prev_index, None);
            }
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
        assert_eq!(pq.head(), None);
        assert_eq!(pq.free(), Some(2));
    }

    #[test]
    fn test_pop() {
        let mut pq = PriorityQueue::<5, i32> {
            data: [
                MaybeUninit::uninit(),
                MaybeUninit::uninit(),
                MaybeUninit::new(1),
                MaybeUninit::new(2),
                MaybeUninit::new(3),
            ],
            next: [
                Some(2), // head
                None,    // free
                Some(3),
                Some(4),
                None,
            ],
            prev: 0,
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

            assert_eq!(PQ.insert(3, MockTest::None), Ok(()));
            println!("{:?}", PQ);
            assert_eq!(PQ.peek(), Some(3));
            assert_eq!(PQ.head(), Some(2));
            assert_eq!(PQ.free(), Some(3));

            assert_eq!(PQ.insert(2, MockTest::None), Ok(()));
            println!("{:?}", PQ);
            assert_eq!(PQ.peek(), Some(2));
            assert_eq!(PQ.insert(1, MockTest::None), Ok(()));
            println!("{:?}", PQ);
            assert_eq!(PQ.peek(), Some(1));
            assert_eq!(PQ.insert(0, MockTest::None), Err(Error::QueueFull));
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
    fn test_insert_last() {
        unsafe {
            static mut PQ: PriorityQueue<5, i32> = PriorityQueue::<5, i32>::new();
            println!("{:?}", PQ);
            assert_eq!(PQ.head(), None);
            assert_eq!(PQ.free(), Some(2));
            assert_eq!(PQ.peek(), None);

            assert_eq!(PQ.insert(1, MockTest::None), Ok(()));
            println!("{:?}", PQ);
            assert_eq!(PQ.peek(), Some(1));
            assert_eq!(PQ.head(), Some(2));
            assert_eq!(PQ.free(), Some(3));

            assert_eq!(PQ.insert(2, MockTest::None), Ok(()));
            println!("{:?}", PQ);
            assert_eq!(PQ.peek(), Some(1));
            assert_eq!(PQ.insert(3, MockTest::None), Ok(()));
            println!("{:?}", PQ);
            assert_eq!(PQ.peek(), Some(1));
            assert_eq!(PQ.insert(0, MockTest::None), Err(Error::QueueFull));
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

        assert_eq!(pq.insert(2, MockTest::None), Ok(()));
        println!("{:?}", pq);
        assert_eq!(pq.peek(), Some(2));
        assert_eq!(pq.head(), Some(2));
        assert_eq!(pq.free(), Some(3));

        assert_eq!(pq.insert(4, MockTest::None), Ok(()));
        println!("{:?}", pq);
        assert_eq!(pq.peek(), Some(2));

        assert_eq!(pq.insert(3, MockTest::None), Ok(()));
        println!("{:?}", pq);
        assert_eq!(pq.pop(), Some(2));

        assert_eq!(pq.pop(), Some(3));
        println!("{:?}", pq);

        assert_eq!(pq.pop(), Some(4));
        println!("{:?}", pq);
    }

    #[test]
    fn test_preempt_pop() {
        let mut pq = PriorityQueue::<5, i32>::new();
        println!("{:?}", pq);

        assert_eq!(pq.insert(1, MockTest::None), Ok(()));
        assert_eq!(pq.insert(3, MockTest::Pop), Ok(()));
        println!("{:?}", pq);

        println!("-- pop {:?}", pq.pop());
        println!("{:?}", pq);
    }
}
