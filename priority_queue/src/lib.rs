// #![cfg_attr(not(test), no_std)]
#![allow(static_mut_refs)]
use core::mem::MaybeUninit;

#[derive(Debug, Clone, Copy)]
pub struct PriorityQueue<const N: usize, T: Copy + Clone + PartialOrd> {
    data: [(MaybeUninit<T>, Option<u16>); N],
    head: Option<u16>,
    free: Option<u16>,
}

impl<const N: usize, T: Copy + Clone + PartialOrd> PriorityQueue<N, T> {
    #[inline(always)]
    pub const fn new() -> Self {
        let mut data: [(MaybeUninit<T>, Option<u16>); N] = [(MaybeUninit::uninit(), None); N];

        let mut i = 0;

        while i < N {
            data[i].1 = if i < N - 1 {
                Some((i + 1) as u16)
            } else {
                None
            };
            i += 1;
        }

        Self {
            data,
            head: None,
            free: Some(0),
        }
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        if let Some(i) = self.head {
            let (value, next) = self.data[i as usize];
            self.head = next;
            println!("next {:?}", self.head);
            self.data[i as usize].1 = self.free;

            self.free = Some(i);
            println!("free {:?}", self.free);

            Some(unsafe { value.assume_init() })
        } else {
            None
        }
    }

    #[inline(always)]
    unsafe fn peek_at(&self, index: u16) -> T {
        unsafe { self.data[index as usize].0.assume_init() }
    }

    #[inline(always)]
    pub fn peek(&self) -> Option<T> {
        if let Some(i) = self.head {
            Some(unsafe { self.peek_at(i) })
        } else {
            None
        }
    }

    #[inline(always)]
    fn insert_first(&mut self, value: T, free_index: u16, next: Option<u16>) {
        self.free = self.data[free_index as usize].1; // allocated new node from free list
        self.data[free_index as usize] = (MaybeUninit::new(value), next); // Last node
        self.head = Some(free_index); // Update head to new node
    }

    #[inline(always)]
    fn insert_at(
        &mut self,
        value: T,
        prev_index: u16,
        free_index: u16,
        next: Option<u16>,
    ) -> Result<(), ()> {
        self.free = self.data[free_index as usize].1; // allocated new node from free list
        self.data[free_index as usize] = (MaybeUninit::new(value), next); // Last node
        self.data[prev_index as usize].1 = Some(free_index); // Update previous node to new node
        Ok(())
    }

    #[inline(always)]
    pub fn insert(&mut self, value: T) -> Result<(), ()> {
        // check if free list is not empty
        if let Some(free_index) = self.free {
            // check if list is not empty
            if let Some(head_index) = self.head {
                // list is not empty, find correct position to insert
                if value < self.peek().unwrap() {
                    // less then first element
                    return Ok(self.insert_first(value, free_index, Some(head_index)));
                } else {
                    // find the correct position to insert
                    let mut prev_index = head_index;

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
                    }
                }
            } else {
                // list is empty, insert first node
                return Ok(self.insert_first(value, free_index, None));
            }
        } else {
            Err(())
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
        let pq = PriorityQueue::<3, i32>::new();
        println!("{:?}", pq);
        assert_eq!(pq.head, None);
        assert_eq!(pq.free, Some(0));
    }   

    #[test]
    fn test_pop() {
        let mut pq = PriorityQueue::<3, i32> {
            data: [
                (MaybeUninit::new(1), Some(1)),
                (MaybeUninit::new(2), Some(2)),
                (MaybeUninit::new(3), None),
            ],
            head: Some(0),
            free: None,
        };

        println!("{:?}", pq);
        assert_eq!(pq.pop(), Some(1));
        println!("{:?}", pq);
        assert_eq!(pq.pop(), Some(2));
        println!("{:?}", pq);
        assert_eq!(pq.pop(), Some(3));
        assert_eq!(pq.head, None);
        assert_eq!(pq.free, Some(2));
    }

    #[test]
    fn test_insert_first() {
        unsafe {
            static mut PQ: PriorityQueue<3, i32> = PriorityQueue::<3, i32>::new();
            println!("{:?}", PQ);
            assert_eq!(PQ.head, None);
            assert_eq!(PQ.free, Some(0));
            assert_eq!(PQ.peek(), None);

            assert_eq!(PQ.insert(3), Ok(()));
            println!("{:?}", PQ);
            assert_eq!(PQ.peek(), Some(3));
            assert_eq!(PQ.head, Some(0));
            assert_eq!(PQ.free, Some(1));

            assert_eq!(PQ.insert(2), Ok(()));
            println!("{:?}", PQ);
            assert_eq!(PQ.insert(1), Ok(()));
            println!("{:?}", PQ);
            assert_eq!(PQ.insert(0), Err(()));
            println!("{:?}", PQ);

            assert_eq!(PQ.pop(), Some(1));
            println!("{:?}", PQ);
            assert_eq!(PQ.pop(), Some(2));
            println!("{:?}", PQ);
            assert_eq!(PQ.pop(), Some(3));
            println!("{:?}", PQ);
            assert_eq!(PQ.head, None);
            assert_eq!(PQ.free, Some(0));
            assert_eq!(PQ.pop(), None);
        }
    }

    #[test]
    fn test_insert_middle() {
        let mut pq = PriorityQueue::<3, i32>::new();
        println!("{:?}", pq);

        assert_eq!(pq.insert(2), Ok(()));
        println!("{:?}", pq);
        assert_eq!(pq.peek(), Some(2));
        assert_eq!(pq.head, Some(0));
        assert_eq!(pq.free, Some(1));

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
