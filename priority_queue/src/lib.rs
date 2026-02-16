#![cfg_attr(not(feature = "std"), no_std)]
#![allow(static_mut_refs)]

use core::cell::UnsafeCell;
use core::num::NonZeroU16;
use std::fmt::Debug;

pub(crate) mod mock_cs;
use mock_cs::PreemptionPoint;

pub(crate) mod node;
use node::*;

use crate::mock_cs::CsToken;

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
    T: PartialOrd + Copy,
{
    data: [UnsafeCell<Node<T>>; N],
    // head: Option<u16>,
    // free: Option<u16>,
}

impl<T, const N: usize> Default for PriorityQueue<T, N>
where
    T: PartialOrd + Copy + Debug,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> PriorityQueue<T, N>
where
    T: PartialOrd + Copy + Debug,
{
    const unsafe fn get<CST>(&self, _cs: &CsToken<CST>, idx: usize) -> &Node<T> {
        unsafe { &*self.data[idx].get() }
    }

    #[warn(clippy::mut_from_ref)]
    const unsafe fn get_mut<CST>(&self, _cs: &CsToken<CST>, idx: usize) -> &mut Node<T> {
        unsafe { &mut *self.data[idx].get() }
    }

    #[inline(always)]
    const unsafe fn head<CST>(&self, cs: &CsToken<CST>) -> NodePtr {
        unsafe { self.get(cs, 0).next }
    }

    #[inline(always)]
    const unsafe fn set_head<CST>(&self, cs: &CsToken<CST>, head: NodePtr) {
        unsafe {
            self.get_mut(cs, 0).next = head;
        }
    }

    #[inline(always)]
    const unsafe fn free<CST>(&self, cs: &CsToken<CST>) -> NodePtr {
        unsafe { self.get(cs, 1).next }
    }

    #[inline(always)]
    const unsafe fn set_free<CST>(&self, cs: &CsToken<CST>, free: NodePtr) {
        unsafe {
            self.get_mut(cs, 1).next = free;
        }
    }

    // TODO: safety: should be run either in const context or in a critical section..
    #[inline(always)]
    pub const fn new() -> Self {
        let pq = Self {
            data: [const { UnsafeCell::new(Node::new_empty()) }; N],
        };

        unsafe {
            let noop = CsToken::new(());
            // initialize head
            pq.set_head(&noop, None);

            // initialize free list
            pq.set_free(&noop, NonZeroU16::new(2));

            // It's stupid we can't use for loops in const fns :(
            let mut i = 2;
            while i < N {
                pq.get_mut(&noop, i).next = if i < N - 1 {
                    NonZeroU16::new(i as u16 + 1)
                } else {
                    None
                };
                i += 1;
            }
        }

        pq
    }

    #[inline(always)]
    pub fn pop<CST>(&self, cs: &CsToken<CST>) -> Option<T> {
        unsafe {
            // Return None directly if list is empty
            let i = self.head(cs)?;

            // SAFETY: no need to check, we already verified list isn't empty
            let node = self.get_mut(cs, i.to_usize());

            // Update head
            self.set_head(cs, node.next);

            #[cfg(feature = "std")]
            println!("next {:?}", self.head(cs));

            // If the node to be popped is being read, simply mark it for deletion.
            // Otherwise, update the free list
            if node.is_being_read() {
                node.mark_for_deletion();
            } else {
                node.next = self.free(cs);
                self.set_free(cs, Some(i));

                #[cfg(feature = "std")]
                println!("free {:?}", self.free(cs));
            }

            Some(node.data.assume_init())
        }
    }

    #[inline(always)]
    unsafe fn peek_at<CST>(&self, cs: &CsToken<CST>, index: NonZeroU16) -> T {
        unsafe { self.get(cs, index.to_usize()).data.assume_init() }
    }

    // TODO: this should be lock-protected?
    #[inline(always)]
    pub fn peek<CST>(&self, cs: &CsToken<CST>) -> Option<T> {
        unsafe { self.head(cs).map(|i| self.peek_at(cs, i)) }
    }

    #[inline(always)]
    fn insert_first<CST>(
        &self,
        cs: &CsToken<CST>,
        value: T,
        free_index: NonZeroU16,
        next: NodePtr,
    ) {
        unsafe {
            let new = self.get_mut(cs, free_index.to_usize());
            // Allocated new node from free list
            self.set_free(cs, new.next);

            // Last node
            *new = Node::new(value, next);

            // Update head to new node
            self.set_head(cs, Some(free_index));
        }
    }

    #[inline(always)]
    fn insert_at<CST>(
        &self,
        cs: &CsToken<CST>,
        value: T,
        prev_index: NonZeroU16,
        free_index: NonZeroU16,
        next: NodePtr,
    ) -> Result<(), Error> {
        unsafe {
            // Allocated new node from free list
            let new = self.get_mut(cs, free_index.to_usize());
            self.set_free(cs, new.next);

            // Last node
            *new = Node::new(value, next);

            // Update previous node to new node
            self.get_mut(cs, prev_index.to_usize()).next = Some(free_index);

            Ok(())
        }
    }

    #[inline(always)]
    pub fn insert<CSA, CST>(&self, cs: &mut CsToken<CST>, value: T) -> Result<(), Error>
    where
        CSA: PreemptionPoint<Inner = CST>,
    {
        unsafe {
            // check if free list is not empty
            let Some(free_index) = self.free(cs) else {
                return Err(Error::QueueFull);
            };

            // check if list is not empty
            let Some(head_index) = self.head(cs) else {
                // list is empty, insert first node
                self.insert_first(cs, value, free_index, None);
                return Ok(());
            };

            // List is not empty, find correct position to insert
            // TODO: can unwrap_unchecked here? We've confirmed that list isn't empty
            if value < self.peek(cs).unwrap() {
                // less then first element
                self.insert_first(cs, value, free_index, Some(head_index));
                Ok(())
            } else {
                // find the correct position to insert
                let mut prev_index = head_index;

                // mock
                loop {
                    // check if last node
                    match self.get(cs, prev_index.to_usize()).next {
                        None => {
                            // we reached the end of the list, insert at the end
                            return self.insert_at(cs, value, prev_index, free_index, None);
                        }

                        Some(next_index) => {
                            if value < self.peek_at(cs, next_index) {
                                // smaller than next node,
                                return self.insert_at(
                                    cs,
                                    value,
                                    prev_index,
                                    free_index,
                                    Some(next_index),
                                );
                            } else {
                                // move to next node
                                prev_index = next_index;

                                // Increment the reader count before exiting the CS
                                self.get_mut(cs, prev_index.to_usize()).inc_readers();
                            }
                        }
                    }

                    print_data(&self.data);
                    // TODO: what happens if the node at next_index gets popped inside the yield
                    // point?
                    CSA::preemption_point(cs);

                    // Remove ourselves from the readers count
                    let prev_node = self.get_mut(cs, prev_index.to_usize());
                    prev_node.dec_readers();

                    // Garbage-collect the node if needed
                    if prev_node.is_marked_for_deletion() && !prev_node.is_being_read() {
                        #[cfg(feature = "std")]
                        println!("Garbage collect node: {prev_node:?}");

                        prev_node.next = self.free(cs);
                        self.set_free(cs, Some(prev_index));
                    }

                    // If the node was marked for deletion, skip it whether it was garbage-collected or not
                    if prev_node.is_marked_for_deletion() {
                        #[cfg(feature = "std")]
                        println!("Skip node: marked for deletion {prev_node:?}");
                        prev_index = prev_node.next.unwrap();
                    }
                }
            }
        }
    }
}

unsafe fn print_data<T: Debug + Copy + Clone, const N: usize>(data: &[UnsafeCell<Node<T>>; N]) {
    println!("[DATA]");
    for (i, t) in data.iter().enumerate() {
        let i = if i == 0 {
            format!("({i}/head)")
        } else if i == 1 {
            format!("({i}/free)")
        } else {
            format!("({i})")
        };
        println!("\t{i}: {:?}", unsafe { *t.get() });
    }
}

unsafe impl<T, const N: usize> Send for PriorityQueue<T, N> where T: PartialOrd + Copy {}
unsafe impl<T, const N: usize> Sync for PriorityQueue<T, N> where T: PartialOrd + Copy {}

#[cfg(test)]
mod tests {

    use std::time::Duration;

    use crate::mock_cs::{MutexCs, NoopCs};

    use super::*;

    #[test]
    fn test_new() {
        unsafe {
            let cs = &CsToken::new(());

            let pq = PriorityQueue::<i32, 5>::new();
            println!("{:?}", pq);
            assert_eq!(pq.get(cs, 0).next, None);
            assert_eq!(pq.get(cs, 1).next, NonZeroU16::new(2));
        }
    }

    #[test]
    fn test_pop() {
        let pq = PriorityQueue::<i32, 5> {
            data: [
                // Head
                UnsafeCell::new(Node::new_uninit(NonZeroU16::new(2))),
                // Free
                UnsafeCell::new(Node::new_uninit(None)),
                UnsafeCell::new(Node::new(1, NonZeroU16::new(3))),
                UnsafeCell::new(Node::new(2, NonZeroU16::new(4))),
                UnsafeCell::new(Node::new(3, None)),
            ],
            //         head: NonZeroU16::new(0),
            //         free: None,
        };

        let cs = unsafe { &CsToken::new(()) };

        println!("{:?}", pq);
        assert_eq!(pq.pop(cs), Some(1));
        println!("{:?}", pq);
        assert_eq!(pq.pop(cs), Some(2));
        println!("{:?}", pq);
        assert_eq!(pq.pop(cs), Some(3));
        println!("{:?}", pq);

        unsafe {
            assert_eq!(pq.head(cs), None);
            assert_eq!(pq.free(cs), NonZeroU16::new(4));
        }
    }

    #[test]
    fn test_insert_first() {
        unsafe {
            let cs = &mut CsToken::new(());

            static PQ: PriorityQueue<i32, 5> = PriorityQueue::<i32, 5>::new();
            println!("{:?}", PQ);
            assert_eq!(PQ.head(cs), None);
            assert_eq!(PQ.free(cs), NonZeroU16::new(2));
            assert_eq!(PQ.peek(cs), None);

            assert_eq!(PQ.insert::<NoopCs, _>(cs, 3), Ok(()));
            println!("{:?}", PQ);
            assert_eq!(PQ.peek(cs), Some(3));
            assert_eq!(PQ.head(cs), NonZeroU16::new(2));
            assert_eq!(PQ.free(cs), NonZeroU16::new(3));

            assert_eq!(PQ.insert::<NoopCs, _>(cs, 2), Ok(()));
            println!("{:?}", PQ);
            assert_eq!(PQ.peek(cs), Some(2));
            assert_eq!(PQ.insert::<NoopCs, _>(cs, 1), Ok(()));
            println!("{:?}", PQ);
            assert_eq!(PQ.peek(cs), Some(1));
            assert_eq!(PQ.insert::<NoopCs, _>(cs, 0), Err(Error::QueueFull));
            println!("{:?}", PQ);

            assert_eq!(PQ.pop(cs), Some(1));
            println!("{:?}", PQ);
            assert_eq!(PQ.pop(cs), Some(2));
            println!("{:?}", PQ);
            assert_eq!(PQ.pop(cs), Some(3));
            println!("{:?}", PQ);
            assert_eq!(PQ.head(cs), None);
            assert_eq!(PQ.free(cs), NonZeroU16::new(2));
            assert_eq!(PQ.pop(cs), None);
        }
    }

    #[test]
    fn test_insert_middle() {
        unsafe {
            let cs = &mut CsToken::new(());
            let pq = PriorityQueue::<i32, 5>::new();
            println!("{:?}", pq);

            assert_eq!(pq.insert::<NoopCs, _>(cs, 2), Ok(()));
            println!("{:?}", pq);
            assert_eq!(pq.peek(cs), Some(2));
            assert_eq!(pq.head(cs), NonZeroU16::new(2));
            assert_eq!(pq.free(cs), NonZeroU16::new(3));

            assert_eq!(pq.insert::<NoopCs, _>(cs, 4), Ok(()));
            println!("{:?}", pq);
            assert_eq!(pq.peek(cs), Some(2));

            assert_eq!(pq.insert::<NoopCs, _>(cs, 3), Ok(()));
            println!("{:?}", pq);
            assert_eq!(pq.pop(cs), Some(2));

            assert_eq!(pq.pop(cs), Some(3));
            println!("{:?}", pq);

            assert_eq!(pq.pop(cs), Some(4));
            println!("{:?}", pq);
        }
    }

    #[test]
    fn test_gc() {
        use std::thread;

        static PQ: PriorityQueue<i32, 5> = PriorityQueue::<i32, 5>::new();

        PQ.insert::<NoopCs, _>(&mut NoopCs.enter(), 1).unwrap();
        PQ.insert::<NoopCs, _>(&mut NoopCs.enter(), 2).unwrap();

        unsafe {
            print_data(&PQ.data);
        }

        let t1 = thread::spawn(|| {
            let mutex_cs = MutexCs::new(Duration::from_millis(500));
            PQ.insert::<MutexCs, _>(&mut mutex_cs.enter(), 3).unwrap();
        });

        let t2 = thread::spawn(|| {
            let mutex_cs = MutexCs::new(Duration::from_millis(500));
            let popped = PQ.pop(&mutex_cs.enter()).unwrap();
            println!("popped: {popped}");
        });

        t2.join().unwrap();
        t1.join().unwrap();
    }
}
