#![cfg_attr(all(not(test), not(feature = "std")), no_std)]

use core::cell::UnsafeCell;
use core::{fmt::Debug, ptr};

use critical_section::{CriticalSection, acquire, release};

use crate::node::{Node, NodePtr};

mod node;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    // TODO
    // SmallerThanMin,
    QueueFull,
}

// #[derive(Debug)]
pub struct PriorityQueue<T: PartialOrd, const N: usize> {
    data: [UnsafeCell<Node<T>>; N],
    head_ptr: UnsafeCell<Option<NodePtr>>,
    free_ptr: UnsafeCell<Option<NodePtr>>,
    tail_ptr: UnsafeCell<Option<NodePtr>>,
    min_ptr: UnsafeCell<Option<NodePtr>>,
}

impl<T: PartialOrd, const N: usize> Default for PriorityQueue<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl<T: PartialOrd, const N: usize> Send for PriorityQueue<T, N> {}
unsafe impl<T: PartialOrd, const N: usize> Sync for PriorityQueue<T, N> {}

// TODO: remove this ugly-ass impl block when done debugging
impl<T: Debug + PartialOrd, const N: usize> Debug for PriorityQueue<T, N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        critical_section::with(|_| {
            unsafe {
                writeln!(
                    f,
                    "PriorityQueue:\n\thead_ptr = {:?}\n\ttail_ptr = {:?}\n\tfree_ptr = {:?}\n\tmin_ptr = {:?}",
                    self.head_ptr, self.tail_ptr, self.free_ptr, self.min_ptr
                )?;

                writeln!(f, "[STORAGE]")?;

                for i in 0..N {
                    // TODO: the peek_at call here is definitely unsound
                    writeln!(
                        f,
                        "\t({i}) {:?}, value: {:?}",
                        self.data[i],
                        self.peek_at(i as NodePtr)
                    )?;

                    // writeln!(f, "\t({i}) {:?}", self.data[i])?;
                }
                writeln!(f, "[DATA]")?;

                if let Some(mut cursor) = *self.head_ptr.get() {
                    loop {
                        // TODO: the peek_at call here is definitely unsound. Only use for debugging
                        writeln!(
                            f,
                            "\t({cursor}) {:?}, value: {:?}",
                            self.data[cursor as usize],
                            self.peek_at(cursor)
                        )?;
                        // writeln!(f, "\t({cursor}) {:?}", self.data[cursor as usize])?;

                        if let Some(next) = *self.next_at(cursor) {
                            cursor = next
                        } else {
                            break;
                        };
                    }
                }

                writeln!(f, "[FREE]")?;

                if let Some(mut cursor) = *self.free_ptr.get() {
                    loop {
                        // TODO: the peek_at call here is definitely unsound. Only use for debugging
                        writeln!(
                            f,
                            "\t({cursor}) {:?}, value: {:?}",
                            self.data[cursor as usize],
                            self.peek_at(cursor)
                        )?;
                        // writeln!(f, "\t({cursor}) {:?}", self.data[cursor as usize])?;

                        if let Some(next) = *self.next_at(cursor) {
                            cursor = next
                        } else {
                            break;
                        };
                    }
                }
            }
            Ok(())
        })
    }
}

impl<T: PartialOrd, const N: usize> PriorityQueue<T, N> {
    /// Returns a `&` reference to the node held at the specified index.
    ///
    /// # Safety
    ///
    /// The following invariants must be held:
    ///
    /// * The provided index must be within the backing array's bounds. No
    ///   runtime checks are performed.
    #[inline]
    unsafe fn node_at(&self, idx: NodePtr) -> *mut Node<T> {
        unsafe { self.data.get_unchecked(idx as usize).get() }
    }

    /// Returns a reference to the value held at the specified node index.
    ///
    /// # Safety
    ///
    /// The following invariants must be held:
    ///
    /// * The provided index must be within the backing array's bounds. No
    ///   runtime checks are performed.
    /// * The data held by the accessed node *must* have been previously
    ///   initialized.
    /// * The node at the specified index must not already be mutably borrowed
    #[inline]
    unsafe fn peek_at(&self, idx: NodePtr) -> &T {
        unsafe { (&*self.node_at(idx)).value.assume_init_ref() }
    }

    /// Returns a raw pointer Option to the `next` pointer held by the specified
    /// node
    #[inline]
    unsafe fn next_at(&self, idx: NodePtr) -> *mut Option<NodePtr> {
        unsafe { &mut (*self.node_at(idx)).next as _ }
    }

    /// Returns a reference to the node at the head of the free list
    #[inline]
    unsafe fn free_node(&self) -> Option<&Node<T>> {
        // SAFETY: free_ptr is guaranteed to be within the list bounds if it is `Some`
        unsafe { Some(&*self.node_at((self.get_free_ptr())?)) }
    }

    /// Returns a reference to the node at the tail of the list
    #[inline]
    unsafe fn tail_node(&self) -> Option<*mut Node<T>> {
        // SAFETY: tail_ptr is guaranteed to be within the list bounds if it is `Some`
        unsafe { Some(self.node_at(self.get_tail_ptr()?)) }
    }

    #[inline]
    unsafe fn get_tail_ptr(&self) -> Option<NodePtr> {
        unsafe { *self.tail_ptr.get() }
    }

    #[inline]
    unsafe fn set_tail_ptr(&self, new: Option<NodePtr>) {
        unsafe {
            *self.tail_ptr.get() = new;
        }
    }

    #[inline]
    unsafe fn get_min_ptr(&self) -> Option<NodePtr> {
        unsafe { *self.min_ptr.get() }
    }

    #[inline]
    unsafe fn set_min_ptr(&self, new: Option<NodePtr>) {
        unsafe {
            *self.min_ptr.get() = new;
        }
    }

    #[inline]
    unsafe fn get_head_ptr(&self) -> Option<NodePtr> {
        unsafe { *self.head_ptr.get() }
    }

    #[inline]
    unsafe fn set_head_ptr(&self, new: Option<NodePtr>) {
        unsafe {
            *self.head_ptr.get() = new;
        }
    }

    #[inline]
    unsafe fn get_free_ptr(&self) -> Option<NodePtr> {
        unsafe { *self.free_ptr.get() }
    }

    #[inline]
    unsafe fn set_free_ptr(&self, new: Option<NodePtr>) {
        unsafe {
            *self.free_ptr.get() = new;
        }
    }

    /// Create a new queue.
    #[inline]
    pub const fn new() -> Self {
        let mut pq = Self {
            data: [const { UnsafeCell::new(Node::new_uninit()) }; N],
            head_ptr: UnsafeCell::new(None),
            tail_ptr: UnsafeCell::new(None),
            free_ptr: UnsafeCell::new(Some(0)),
            min_ptr: UnsafeCell::new(None),
        };

        // Initialize free list.
        // Annoyingly, we can't use for loops in const fns :(
        let mut i = 0;
        while i < N {
            pq.data[i].get_mut().next = if i < N - 1 {
                Some(i as NodePtr + 1)
            } else {
                None
            };
            i += 1;
        }

        pq
    }

    /// Return a reference to the minimum element in the queue.
    ///
    /// To access the min element by reference, we must be inside a critical
    /// section. For types `T` that are [`Clone`], you can also use
    /// [`min`](PriorityQueue::min).
    #[inline]
    pub fn min_ref<'cs>(&'cs self, _cs: CriticalSection<'cs>) -> Option<&'cs T> {
        // SAFETY: data[min_ptr] is guaranteed to always be initialized if min_ptr is
        // Some
        unsafe { Some(self.peek_at(self.get_min_ptr()?)) }
    }

    /// Insert an element into the queue.
    ///
    /// # Errors
    ///
    /// * Returns [`Error::QueueFull`] if there is no space left in the backing
    ///   storage.
    /// * Returns [`Error::SmallerThanMin`] if attempting to insert an element
    ///   that is smaller than the current minimum in the queue.
    // TODO: implement the above error
    #[inline]
    pub fn insert(&self, data: T) -> Result<(), Error> {
        // Entire node-swapping must be performed atomically
        critical_section::with(|_| {
            unsafe {
                // Pick the first free node to allocate to and move the free ptr to the next
                // available free node
                let insert_at = self.get_free_ptr().ok_or(Error::QueueFull)?;

                // SAFETY: We've just proven free is Some above
                let next_free = self.free_node().unwrap_unchecked().next;
                self.set_free_ptr(next_free);

                match self.tail_node() {
                    Some(t) => {
                        let t = &mut *t;
                        let new_tail = Some(insert_at);
                        t.next = new_tail;
                        self.set_tail_ptr(new_tail);

                        // SAFETY: we are inside a critical section
                        let cs = CriticalSection::new();

                        // Update the global minimum ptr if necessary
                        // SAFETY: min is guaranteed to be Some if tail is Some
                        if data < *self.min_ref(cs).unwrap_unchecked() {
                            self.set_min_ptr(new_tail);
                        }
                    }
                    None => {
                        self.set_head_ptr(Some(0));
                        self.set_tail_ptr(Some(0));
                        self.set_min_ptr(Some(0));
                    }
                }

                // SAFETY: tail is guaranteed to be Some from above
                *self.tail_node().unwrap_unchecked() = Node::new(data, None);

                Ok(())
            }
        })
    }

    #[inline]
    pub fn pop(&self) -> Option<T> {
        struct TraversalState {
            min_ptr: NodePtr,
            second_min_ptr: NodePtr,
            prev_cursor: NodePtr,
            cursor: NodePtr,
            min_predecessor: NodePtr,
        }

        struct State(UnsafeCell<Option<TraversalState>>);
        unsafe impl Sync for State {}

        static STATE: State = State(UnsafeCell::new(None));

        unsafe {
            // SAFETY: Cannot use critical_section::with because returning from the closure
            // doesn't return the entire function. We have to be careful to release the CS
            // at every point where the function can return.
            let cs_restore = acquire();

            // First, check whether STATE is full or empty. If full, this means we're
            // preempting/stealing an ongoing pop operation; we simply move onto the next
            // step. If empty, we start a new one.
            let state = &mut *STATE.0.get();
            if state.is_none() {
                // List is empty
                let Some(head_ptr) = *self.head_ptr.get() else {
                    release(cs_restore);
                    return None;
                };

                let head_node = self.node_at(head_ptr);

                // Prepare the cursors which keep track of global minimum and second minimum, if there
                // are at least 2 elements in list
                let (min_ptr, second_min_ptr) = if let Some(next_after_head) = (*head_node).next {
                    if self.peek_at(head_ptr) <= self.peek_at(next_after_head) {
                        (head_ptr, next_after_head)
                    } else {
                        (next_after_head, head_ptr)
                    }
                } else {
                    // Otherwise, special case for a singleton list
                    let value = ptr::read((*head_node).value.assume_init_ref());

                    *self.next_at(head_ptr) = self.get_free_ptr();

                    self.set_free_ptr(Some(head_ptr));

                    self.set_head_ptr(None);
                    self.set_tail_ptr(None);
                    self.set_min_ptr(None);

                    release(cs_restore);
                    return Some(value);
                };

                state.replace(TraversalState {
                    min_ptr,
                    second_min_ptr,
                    cursor: head_ptr,
                    prev_cursor: head_ptr,
                    min_predecessor: head_ptr,
                });
            }

            release(cs_restore);

            let (cs_restore, state) = loop {
                let cs_restore = acquire();

                // If state is now None, we've been preempted and the pop has been stolen from
                // under us. Our work here is done.
                let Some(state) = &mut *STATE.0.get() else {
                    critical_section::release(cs_restore);
                    return None;
                };

                // We've reached the end of the queue. Don't release the CS yet; we need to
                // update the queue pointers and pop the task first.
                let Some(next) = *self.next_at(state.cursor) else {
                    break (cs_restore, state);
                };

                // NOTE: <= necessary here to properly handle duplicate elements in list, ie set
                // second_min_ptr to an element of same value as min_value
                if self.peek_at(next) <= self.peek_at(state.min_ptr) && state.min_ptr != next {
                    state.second_min_ptr = state.min_ptr;
                    state.min_ptr = next;
                    state.min_predecessor = state.cursor;
                }

                state.prev_cursor = state.cursor;
                state.cursor = next;

                release(cs_restore);
            };

            let popped_value = ptr::read(self.peek_at(state.min_ptr));
            let next_after_min = *self.next_at(state.min_ptr);

            // If popped node was head, update head
            if Some(state.min_ptr) == self.get_head_ptr() {
                self.set_head_ptr(next_after_min);
            } else {
                // Otherwise patch previous node
                (*self.node_at(state.min_predecessor)).next = next_after_min;
            }

            // If popped node was tail, update tail
            if Some(state.min_ptr) == self.get_tail_ptr() {
                self.set_tail_ptr(Some(state.prev_cursor));
            }

            // Deallocate node by moving it into the free list
            *self.next_at(state.min_ptr) = self.get_free_ptr();
            self.set_free_ptr(Some(state.min_ptr));

            // Update new cached queue minimum
            self.set_min_ptr(Some(state.second_min_ptr));

            (*STATE.0.get()) = None;
            release(cs_restore);
            Some(popped_value)
        }
    }
}

impl<T: PartialOrd + Clone, const N: usize> PriorityQueue<T, N> {
    /// Return the minimum element in the queue by value.
    ///
    /// To access the min element by reference, you can also use
    /// [`min_ref`](PriorityQueue::min_ref).
    #[inline]
    pub fn min(&self) -> Option<T> {
        // SAFETY: data[min_ptr] is guaranteed to always be initialized if min_ptr is
        // Some
        critical_section::with(|_| unsafe {
            let min = self.peek_at(self.get_min_ptr()?);
            Some(min.clone())
        })
    }
}

#[cfg(test)]
mod tests;
