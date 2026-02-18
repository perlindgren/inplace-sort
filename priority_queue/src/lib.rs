#![cfg_attr(all(not(test), not(feature = "std")), no_std)]

use crate::node::{Node, NodePtr};

mod cs_mutex;
mod node;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    SmallerThanMin,
    QueueFull,
}

// #[derive(Debug)]
pub struct PriorityQueue<T: PartialOrd, const N: usize> {
    data: [Node<T>; N],
    head_ptr: Option<NodePtr>,
    free_ptr: Option<NodePtr>,
    tail_ptr: Option<NodePtr>,
    min_ptr: Option<NodePtr>,
}

// TODO: remove Clone bounds
impl<T: PartialOrd + Clone + core::fmt::Debug, const N: usize> Default for PriorityQueue<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl<T: PartialOrd, const N: usize> Send for PriorityQueue<T, N> {}
unsafe impl<T: PartialOrd, const N: usize> Sync for PriorityQueue<T, N> {}

impl<T: core::fmt::Debug + PartialOrd, const N: usize> core::fmt::Debug for PriorityQueue<T, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(
            f,
            "PriorityQueue:\n\thead_ptr = {:?}\n\ttail_ptr = {:?}\n\tfree_ptr = {:?}\n\tmin_ptr = {:?}",
            self.head_ptr, self.tail_ptr, self.free_ptr, self.min_ptr
        )?;

        writeln!(f, "[STORAGE]")?;

        for i in 0..N {
            // TODO: the unsafe block here is definitely unsound
            writeln!(f, "\t({i}) {:?}, value: {:?}", self.data[i], unsafe {
                self.data[i].value.assume_init_ref()
            })?;

            // writeln!(f, "\t({i}) {:?}", self.data[i])?;
        }
        writeln!(f, "[DATA]")?;

        if let Some(mut cursor) = self.head_ptr {
            loop {
                // TODO: the unsafe block here is definitely unsound
                writeln!(
                    f,
                    "\t({cursor}) {:?}, value: {:?}",
                    self.data[cursor as usize],
                    unsafe { self.data[cursor as usize].value.assume_init_ref() }
                )?;
                // writeln!(f, "\t({cursor}) {:?}", self.data[cursor as usize])?;

                if let Some(next) = self.data[cursor as usize].next {
                    cursor = next
                } else {
                    break;
                };
            }
        }

        writeln!(f, "[FREE]")?;

        if let Some(mut cursor) = self.free_ptr {
            loop {
                // TODO: the unsafe block here is definitely unsound
                writeln!(
                    f,
                    "\t({cursor}) {:?}, value: {:?}",
                    self.data[cursor as usize],
                    unsafe { self.data[cursor as usize].value.assume_init_ref() }
                )?;
                // writeln!(f, "\t({cursor}) {:?}", self.data[cursor as usize])?;

                if let Some(next) = self.data[cursor as usize].next {
                    cursor = next
                } else {
                    break;
                };
            }
        }

        Ok(())
    }
}

impl<T: PartialOrd + Clone + core::fmt::Debug, const N: usize> PriorityQueue<T, N> {
    #[inline]
    fn peek_at(&self, idx: NodePtr) -> Option<&Node<T>> {
        self.data.get(idx as usize)
    }

    #[inline]
    fn peek_at_mut(&mut self, idx: NodePtr) -> Option<&mut Node<T>> {
        self.data.get_mut(idx as usize)
    }

    #[inline]
    fn free(&self) -> Option<&Node<T>> {
        self.peek_at(self.free_ptr?)
    }

    #[inline]
    fn head(&self) -> Option<&Node<T>> {
        self.peek_at(self.head_ptr?)
    }

    #[inline]
    fn tail_mut(&mut self) -> Option<&mut Node<T>> {
        self.peek_at_mut(self.tail_ptr?)
    }

    #[inline]
    pub const fn new() -> Self {
        let mut pq = Self {
            data: [const { Node::new_uninit() }; N],
            head_ptr: None,
            tail_ptr: None,
            free_ptr: Some(0),
            min_ptr: None,
        };

        // Initialize free list.
        // Annoyingly, we can't use for loops in const fns :(
        let mut i = 0;
        while i < N {
            pq.data[i].next = if i < N - 1 { Some(i as u16 + 1) } else { None };
            i += 1;
        }

        pq
    }

    #[inline]
    pub fn min(&self) -> Option<&T> {
        critical_section::with(|_| {
            // SAFETY: data[min_ptr] is guaranteed to always be initialized if min_ptr is
            // Some
            unsafe {
                self.data
                    .get(self.min_ptr? as usize)
                    .map(|n| n.value.assume_init_ref())
            }
        })
    }

    /// Insert an element into the queue.
    ///
    /// # Errors
    ///
    ///  * Returns [`Error::QueueFull`] if there is no space left in the backing
    ///    storage.
    /// * Returns [`Error::SmallerThanMin`] if attempting to insert an element
    ///   that is smaller than the current minimum in the queue.
    // TODO: implement the above error
    #[inline]
    pub fn insert(&mut self, data: T) -> Result<(), Error> {
        // Entire node-swapping must be performed atomically
        critical_section::with(|_| {
            // Pick the first free node to allocate to and move the free ptr to the next
            // available free node
            let insert_at = self.free_ptr.ok_or(Error::QueueFull)?;

            // SAFETY: We've just proven free is Some above
            self.free_ptr = unsafe { self.free().unwrap_unchecked().next };

            match self.tail_mut() {
                Some(t) => {
                    t.next = Some(insert_at);
                    self.tail_ptr = Some(insert_at);
                    // SAFETY: don't need to check the unwrap,min is guaranteed to be Some if tail
                    // is Some Update the global queue minimum if necessary
                    unsafe {
                        if data < *self.min().unwrap_unchecked() {
                            self.min_ptr = self.tail_ptr;
                        }
                    }
                }
                None => {
                    self.head_ptr = Some(0);
                    self.tail_ptr = Some(0);
                    self.min_ptr = Some(0);
                }
            }

            // SAFETY: tail is guaranteed to be set above
            unsafe {
                *self.tail_mut().unwrap_unchecked() = Node::new(data, None);
            }

            Ok(())
        })
    }

    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        unsafe {
            let head_ptr = self.head_ptr?;

            let head_node = self.head().unwrap_unchecked();

            // Seed cursors which keep track of global minimum and second minimum if there
            // are at least 2 elements in list
            let (mut min_ptr, mut second_min_ptr) = if let Some(second_ptr) = head_node.next {
                let first_value = self
                    .peek_at(head_ptr)
                    .unwrap_unchecked()
                    .value
                    .assume_init_ref();

                let second_value = self
                    .peek_at(second_ptr)
                    .unwrap_unchecked()
                    .value
                    .assume_init_ref();

                if first_value <= second_value {
                    (head_ptr, second_ptr)
                } else {
                    (second_ptr, head_ptr)
                }
            } else {
                // Otherwise, singleton list special case
                let value = head_node.value.assume_init_ref().clone();

                self.peek_at_mut(head_ptr).unwrap_unchecked().next = self.free_ptr;

                self.free_ptr = Some(head_ptr);

                self.head_ptr = None;
                self.tail_ptr = None;
                self.min_ptr = None;

                return Some(value);
            };

            let mut prev_cursor = head_ptr;
            let mut cursor = head_ptr;
            let mut min_predecessor = head_ptr;

            // SAFETY: node is guaranteed to be Some as check above
            while let Some(next) = self.peek_at(cursor).unwrap_unchecked().next {
                // SAFETY: next has already been checked to be Some, and any node being pointed
                // to has already been initialized
                let next_value = self
                    .peek_at(next)
                    .unwrap_unchecked()
                    .value
                    .assume_init_ref();

                // SAFETY: Any node being pointed to has already been initialized
                let min_value = self.peek_at(min_ptr).unwrap().value.assume_init_ref();

                // NOTE: <= necessary here to properly handle duplicate elements in list, ie set
                // the second_min_ptr to an element of same value as min_value
                if next_value <= min_value && min_ptr != next {
                    second_min_ptr = min_ptr;
                    min_ptr = next;
                    min_predecessor = cursor;
                }

                prev_cursor = cursor;
                cursor = next;
            }

            let removed_value = self
                .peek_at(min_ptr)
                .unwrap_unchecked()
                .value
                .assume_init_ref()
                .clone();

            let next_after_min = self.peek_at(min_ptr).unwrap_unchecked().next;

            // If min is head, update head
            if Some(min_ptr) == self.head_ptr {
                self.head_ptr = next_after_min;
            } else {
                // Patch previous node
                self.peek_at_mut(min_predecessor).unwrap_unchecked().next = next_after_min;
            }

            // If min was tail, update tail
            if Some(min_ptr) == self.tail_ptr {
                self.tail_ptr = Some(prev_cursor);
            }

            // Deallocate node by moving it into the free list
            self.peek_at_mut(min_ptr).unwrap_unchecked().next = self.free_ptr;
            self.free_ptr = Some(min_ptr);

            // Update cached minimum
            self.min_ptr = Some(second_min_ptr);

            Some(removed_value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_tail<T: PartialOrd, const N: usize>(pq: &PriorityQueue<T, N>, idx: usize) {
        assert_eq!(pq.tail_ptr, Some(idx as u16));
        assert_eq!(pq.data[idx].next, None);
    }

    #[test]
    fn new() {
        let pq = PriorityQueue::<i32, 5>::new();
        assert_eq!(pq.head_ptr, None);
        assert_eq!(pq.free_ptr, Some(0));
        assert_eq!(pq.min(), None);
    }

    #[test]
    fn cached_min_remains_in_sync() {
        let mut pq = PriorityQueue::<i32, 5>::new();
        assert_eq!(pq.min(), None);

        // Insert a bunch of data...
        pq.insert(2).unwrap();
        assert_eq!(pq.min(), Some(&2));

        pq.insert(1).unwrap();
        // Did the global min get updated?
        assert_eq!(pq.min(), Some(&1));

        pq.insert(3).unwrap();
        pq.insert(4).unwrap();

        // Is the global min still 1?
        assert_eq!(pq.min(), Some(&1));

        pq.insert(0).unwrap();
        // Global min should now reflect new insert
        assert_eq!(pq.min(), Some(&0));

        // List is full
        assert!(pq.insert(2).is_err());

        // Now let's pop it
        assert_eq!(pq.pop(), Some(0));
        // New min should be 1 again
        assert_eq!(pq.min(), Some(&1));

        assert_eq!(pq.pop(), Some(1));
        assert_eq!(pq.min(), Some(&2));

        assert_eq!(pq.pop(), Some(2));
        assert_eq!(pq.min(), Some(&3));

        assert_eq!(pq.pop(), Some(3));
        assert_eq!(pq.min(), Some(&4));

        assert_eq!(pq.pop(), Some(4));
        assert!(pq.min().is_none());
        assert!(pq.pop().is_none());
    }

    #[test]
    fn pop_length_one_list() {
        let mut pq = PriorityQueue::<i32, 5>::new();

        pq.insert(100).unwrap();
        assert_eq!(pq.head_ptr, Some(0));
        assert_eq!(pq.tail_ptr, Some(0));
        assert_eq!(pq.min_ptr, Some(0));
        assert_eq!(pq.free_ptr, Some(1));

        let min = pq.min();
        assert_eq!(min, Some(&100));

        let popped = pq.pop();
        assert_eq!(popped, Some(100));
        assert_eq!(pq.head_ptr, None);
        assert_eq!(pq.tail_ptr, None);
        assert_eq!(pq.min_ptr, None);
        assert_eq!(pq.free_ptr, Some(0));
    }

    #[test]
    fn pop_length_two_list_ordered() {
        let mut pq = PriorityQueue::<i32, 5>::new();

        pq.insert(100).unwrap();
        pq.insert(200).unwrap();

        assert_eq!(pq.head_ptr, Some(0));
        assert_eq!(pq.tail_ptr, Some(1));
        assert_eq!(pq.min_ptr, Some(0));
        assert_eq!(pq.free_ptr, Some(2));

        let min = pq.min();
        assert_eq!(min, Some(&100));

        let popped = pq.pop();
        assert_eq!(popped, Some(100));

        let min = pq.min();
        assert_eq!(min, Some(&200));

        assert_eq!(pq.head_ptr, Some(1));
        assert_eq!(pq.min_ptr, Some(1));
        assert_tail(&pq, 1);
        // Verify edges of free list
        assert_eq!(pq.free_ptr, Some(0));
        assert_eq!(pq.data[0].next, Some(2));
        assert_eq!(pq.data[4].next, None);
    }

    #[test]
    fn pop_length_two_list_reverse_ordered() {
        let mut pq = PriorityQueue::<i32, 5>::new();

        pq.insert(200).unwrap();
        pq.insert(100).unwrap();

        assert_eq!(pq.head_ptr, Some(0));
        assert_eq!(pq.tail_ptr, Some(1));
        assert_eq!(pq.min_ptr, Some(1));
        assert_eq!(pq.free_ptr, Some(2));

        let min = pq.min();
        assert_eq!(min, Some(&100));

        let popped = pq.pop();
        assert_eq!(popped, Some(100));

        let min = pq.min();
        assert_eq!(min, Some(&200));

        assert_eq!(pq.head_ptr, Some(0));
        assert_eq!(pq.min_ptr, Some(0));
        assert_tail(&pq, 0);
        // Verify edges of free list
        assert_eq!(pq.free_ptr, Some(1));
        assert_eq!(pq.data[1].next, Some(2));
        assert_eq!(pq.data[4].next, None);
    }

    #[test]
    fn pop_end() {
        let mut pq = PriorityQueue::<i32, 5>::new();

        // Arrange test
        pq.insert(2).unwrap();
        assert_eq!(pq.min(), Some(&2));

        pq.insert(1).unwrap();
        // Did the global min get updated?
        assert_eq!(pq.min(), Some(&1));

        pq.insert(3).unwrap();

        pq.insert(4).unwrap();
        // Is the global min still 1?
        assert_eq!(pq.min(), Some(&1));

        pq.insert(0).unwrap();
        // Global min should now reflect new insert
        assert_eq!(pq.min(), Some(&0));

        // List is full
        assert!(pq.insert(2).is_err());

        // Now let's pop it
        assert_eq!(pq.pop(), Some(0));
        assert_eq!(pq.head_ptr, Some(0));
        assert_eq!(pq.min_ptr, Some(1));
        assert_tail(&pq, 3);
        // Verify edges of free list
        assert_eq!(pq.free_ptr, Some(4));
        assert_eq!(pq.data[4].next, None);

        // Check other pops for good measure, without checking the internal state. More
        // popping tests await
        assert_eq!(pq.pop(), Some(1));
        assert_eq!(pq.pop(), Some(2));
    }

    #[test]
    fn duplicate_values() {
        let mut pq = PriorityQueue::<i32, 5>::new();

        pq.insert(100).unwrap();
        pq.insert(200).unwrap();
        pq.insert(100).unwrap();

        let min = pq.min();
        assert_eq!(min, Some(&100));

        let popped = pq.pop();
        assert_eq!(popped, Some(100));
        assert_eq!(pq.min(), Some(&100));

        let popped = pq.pop();
        assert_eq!(popped, Some(100));
        assert_eq!(pq.min(), Some(&200));

        let popped = pq.pop();
        assert_eq!(popped, Some(200));
        assert!(pq.min().is_none());
        assert!(pq.pop().is_none());
    }

    #[test]
    fn pop_middle() {
        let mut pq = PriorityQueue::<i32, 7>::new();

        // Arrange test
        pq.insert(1).unwrap();
        assert_eq!(pq.min(), Some(&1));

        pq.insert(2).unwrap();
        assert_eq!(pq.min(), Some(&1));

        pq.insert(0).unwrap();
        assert_eq!(pq.min(), Some(&0));

        pq.insert(4).unwrap();
        // Is the global min still 1?
        assert_eq!(pq.min(), Some(&0));

        pq.insert(3).unwrap();
        // Is the global min still 1?
        assert_eq!(pq.min(), Some(&0));

        pq.insert(-1).unwrap();
        // Global min should now reflect new insert
        assert_eq!(pq.min(), Some(&-1));

        pq.insert(0).unwrap();
        // Global min should now reflect new insert
        assert_eq!(pq.min(), Some(&-1));

        // ------

        // Now let's pop it
        let popped = pq.pop();
        assert_eq!(popped, Some(-1));
        assert_eq!(pq.min(), Some(&0));

        assert_eq!(pq.head_ptr, Some(0));
        assert_eq!(pq.min_ptr, Some(2));
        assert_tail(&pq, 6);
        // Verify edges of free list
        assert_eq!(pq.free_ptr, Some(5));
        assert_eq!(pq.data[5].next, None);

        // ------

        // Check other pops for good measure
        let popped = pq.pop();
        assert_eq!(popped, Some(0));
        assert_eq!(pq.min(), Some(&0));

        assert_eq!(pq.head_ptr, Some(0));
        assert_eq!(pq.min_ptr, Some(2));
        assert_tail(&pq, 4);
        // Verify edges of free list
        assert_eq!(pq.free_ptr, Some(6));
        assert_eq!(pq.data[5].next, None);

        // ------

        let popped = pq.pop();
        assert_eq!(popped, Some(0));
        assert_eq!(pq.min(), Some(&1));

        assert_eq!(pq.head_ptr, Some(0));
        assert_eq!(pq.min_ptr, Some(0));
        assert_tail(&pq, 4);
        // Verify edges of free list
        assert_eq!(pq.free_ptr, Some(2));
        assert_eq!(pq.data[5].next, None);

        // ------

        // This here pops the head
        let popped = pq.pop();
        assert_eq!(popped, Some(1));
        assert_eq!(pq.min(), Some(&2));

        assert_eq!(pq.head_ptr, Some(1));
        assert_eq!(pq.min_ptr, Some(1));
        assert_tail(&pq, 4);
        // Verify edges of free list
        assert_eq!(pq.free_ptr, Some(0));
        assert_eq!(pq.data[5].next, None);
    }
}
