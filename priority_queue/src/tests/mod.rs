use crate::{PriorityQueue, node::NodePtr};

fn assert_next<T: PartialOrd, const N: usize>(
    pq: &mut PriorityQueue<T, N>,
    idx: NodePtr,
    next: Option<NodePtr>,
) {
    unsafe {
        assert_eq!(*pq.next_at(idx), next);
    }
}

// SAFETY: must be run inside a critical section
fn assert_tail<T: PartialOrd, const N: usize>(pq: &mut PriorityQueue<T, N>, idx: NodePtr) {
    assert_eq!(*pq.tail_ptr.get_mut(), Some(idx));
    assert_next(pq, idx, None);
}

#[cfg_attr(not(loom), test)]
fn new() {
    let mut pq = PriorityQueue::<i32, 5>::new();

    assert_eq!(*pq.head_ptr.get_mut(), None);
    assert_eq!(*pq.free_ptr.get_mut(), Some(0));

    assert_eq!(pq.min(), None);
}

#[cfg(loom)]
#[cfg_attr(loom, test)]
fn concurrent_new() {
    loom::model(|| new());
}

#[cfg_attr(not(loom), test)]
fn cached_min_remains_in_sync() {
    let pq = PriorityQueue::<i32, 5>::new();
    assert_eq!(pq.min(), None);

    // Insert a bunch of data...
    pq.insert(2).unwrap();
    assert_eq!(pq.min(), Some(2));

    pq.insert(1).unwrap();
    // Did the global min get updated?
    assert_eq!(pq.min(), Some(1));

    pq.insert(3).unwrap();
    pq.insert(4).unwrap();

    // Is the global min still 1?
    assert_eq!(pq.min(), Some(1));

    pq.insert(0).unwrap();
    // Global min should now reflect new insert
    assert_eq!(pq.min(), Some(0));

    // List is full
    assert!(pq.insert(2).is_err());

    // Now let's pop it
    assert_eq!(pq.pop(), Some(0));
    // New min should be 1 again
    assert_eq!(pq.min(), Some(1));

    assert_eq!(pq.pop(), Some(1));
    assert_eq!(pq.min(), Some(2));

    assert_eq!(pq.pop(), Some(2));
    assert_eq!(pq.min(), Some(3));

    assert_eq!(pq.pop(), Some(3));
    assert_eq!(pq.min(), Some(4));

    assert_eq!(pq.pop(), Some(4));
    assert!(pq.min().is_none());
    assert!(pq.pop().is_none());
}

#[cfg(loom)]
#[cfg_attr(loom, test)]
fn concurrent_cached_min_remains_in_sync() {
    loom::model(|| cached_min_remains_in_sync());
}

#[cfg_attr(not(loom), test)]
fn pop_length_one_list() {
    let mut pq = PriorityQueue::<i32, 5>::new();

    pq.insert(100).unwrap();

    assert_eq!(*pq.head_ptr.get_mut(), Some(0));
    assert_eq!(*pq.tail_ptr.get_mut(), Some(0));
    assert_eq!(*pq.min_ptr.get_mut(), Some(0));
    assert_eq!(*pq.free_ptr.get_mut(), Some(1));

    // Test min_ref for fun
    critical_section::with(|cs| {
        let min = pq.min_ref(cs);
        assert_eq!(min, Some(&100));
    });

    let min = pq.min();
    assert_eq!(min, Some(100));

    let popped = pq.pop();
    assert_eq!(popped, Some(100));

    assert_eq!(*pq.head_ptr.get_mut(), None);
    assert_eq!(*pq.tail_ptr.get_mut(), None);
    assert_eq!(*pq.min_ptr.get_mut(), None);
    assert_eq!(*pq.free_ptr.get_mut(), Some(0));
}

#[cfg(loom)]
#[cfg_attr(loom, test)]
fn concurrent_pop_length_one_list() {
    loom::model(|| pop_length_one_list());
}

#[cfg_attr(not(loom), test)]
fn pop_length_two_list_ordered() {
    let mut pq = PriorityQueue::<i32, 5>::new();

    pq.insert(100).unwrap();
    pq.insert(200).unwrap();

    assert_eq!(*pq.head_ptr.get_mut(), Some(0));
    assert_eq!(*pq.tail_ptr.get_mut(), Some(1));
    assert_eq!(*pq.min_ptr.get_mut(), Some(0));
    assert_eq!(*pq.free_ptr.get_mut(), Some(2));

    let min = pq.min();
    assert_eq!(min, Some(100));

    let popped = pq.pop();
    assert_eq!(popped, Some(100));

    let min = pq.min();
    assert_eq!(min, Some(200));

    assert_eq!(*pq.head_ptr.get_mut(), Some(1));
    assert_eq!(*pq.min_ptr.get_mut(), Some(1));
    assert_tail(&mut pq, 1);
    // Verify edges of free list
    assert_eq!(*pq.free_ptr.get_mut(), Some(0));
    assert_next(&mut pq, 0, Some(2));
    assert_next(&mut pq, 4, None);

    let popped = pq.pop();
    assert_eq!(popped, Some(200));
}

#[cfg(loom)]
#[cfg_attr(loom, test)]
fn concurrent_pop_length_two_list_ordered() {
    loom::model(|| pop_length_two_list_ordered());
}

#[cfg_attr(not(loom), test)]
fn pop_length_two_list_reverse_ordered() {
    let mut pq = PriorityQueue::<i32, 5>::new();

    pq.insert(200).unwrap();
    pq.insert(100).unwrap();

    assert_eq!(*pq.head_ptr.get_mut(), Some(0));
    assert_eq!(*pq.tail_ptr.get_mut(), Some(1));
    assert_eq!(*pq.min_ptr.get_mut(), Some(1));
    assert_eq!(*pq.free_ptr.get_mut(), Some(2));

    let min = pq.min();
    assert_eq!(min, Some(100));

    let popped = pq.pop();
    assert_eq!(popped, Some(100));

    let min = pq.min();
    assert_eq!(min, Some(200));

    assert_eq!(*pq.head_ptr.get_mut(), Some(0));
    assert_eq!(*pq.min_ptr.get_mut(), Some(0));
    assert_tail(&mut pq, 0);
    // Verify edges of free list
    assert_eq!(*pq.free_ptr.get_mut(), Some(1));
    assert_next(&mut pq, 1, Some(2));
    assert_next(&mut pq, 4, None);

    let popped = pq.pop();
    assert_eq!(popped, Some(200));
}

#[cfg(loom)]
#[cfg_attr(loom, test)]
fn concurrent_pop_length_two_list_reverse_ordered() {
    loom::model(|| pop_length_two_list_reverse_ordered());
}

#[cfg_attr(not(loom), test)]
fn pop_end() {
    let mut pq = PriorityQueue::<i32, 5>::new();

    // Arrange test
    pq.insert(2).unwrap();
    assert_eq!(pq.min(), Some(2));

    pq.insert(1).unwrap();
    // Did the global min get updated?
    assert_eq!(pq.min(), Some(1));

    pq.insert(3).unwrap();

    pq.insert(4).unwrap();
    // Is the global min still 1?
    assert_eq!(pq.min(), Some(1));

    pq.insert(0).unwrap();
    // Global min should now reflect new insert
    assert_eq!(pq.min(), Some(0));

    // List is full
    assert!(pq.insert(2).is_err());

    // Now let's pop it
    assert_eq!(pq.pop(), Some(0));

    assert_eq!(*pq.head_ptr.get_mut(), Some(0));
    assert_eq!(*pq.min_ptr.get_mut(), Some(1));
    assert_tail(&mut pq, 3);
    // Verify edges of free list
    assert_eq!(*pq.free_ptr.get_mut(), Some(4));
    assert_next(&mut pq, 4, None);

    // Test min_ref for fun
    critical_section::with(|cs| {
        let min = pq.min_ref(cs);
        assert_eq!(min, Some(&1));
    });

    // Check other pops for good measure, without checking the internal state. More
    // popping tests await
    assert_eq!(pq.pop(), Some(1));
    assert_eq!(pq.pop(), Some(2));
}

#[cfg(loom)]
#[cfg_attr(loom, test)]
fn concurrent_pop_end() {
    loom::model(|| pop_end());
}

#[cfg_attr(not(loom), test)]
fn duplicate_values() {
    let pq = PriorityQueue::<i32, 5>::new();

    pq.insert(100).unwrap();
    pq.insert(200).unwrap();
    pq.insert(100).unwrap();

    let min = pq.min();
    assert_eq!(min, Some(100));

    let popped = pq.pop();
    assert_eq!(popped, Some(100));
    assert_eq!(pq.min(), Some(100));

    let popped = pq.pop();
    assert_eq!(popped, Some(100));
    assert_eq!(pq.min(), Some(200));

    let popped = pq.pop();
    assert_eq!(popped, Some(200));
    assert!(pq.min().is_none());
    assert!(pq.pop().is_none());
}

#[cfg(loom)]
#[cfg_attr(loom, test)]
fn concurrent_duplicate_values() {
    loom::model(|| duplicate_values());
}

#[cfg_attr(not(loom), test)]
fn pop_middle() {
    let mut pq = PriorityQueue::<i32, 7>::new();

    // Arrange test
    pq.insert(1).unwrap();
    assert_eq!(pq.min(), Some(1));

    pq.insert(2).unwrap();
    assert_eq!(pq.min(), Some(1));

    pq.insert(0).unwrap();
    assert_eq!(pq.min(), Some(0));

    pq.insert(4).unwrap();
    // Is the global min still 1?
    assert_eq!(pq.min(), Some(0));

    pq.insert(3).unwrap();
    // Is the global min still 1?
    assert_eq!(pq.min(), Some(0));

    pq.insert(-1).unwrap();
    // Global min should now reflect new insert
    assert_eq!(pq.min(), Some(-1));

    pq.insert(0).unwrap();
    // Global min should now reflect new insert
    assert_eq!(pq.min(), Some(-1));

    // ------

    // Now let's pop it
    let popped = pq.pop();
    assert_eq!(popped, Some(-1));
    assert_eq!(pq.min(), Some(0));

    assert_eq!(*pq.head_ptr.get_mut(), Some(0));
    assert_eq!(*pq.min_ptr.get_mut(), Some(2));
    assert_tail(&mut pq, 6);
    // Verify edges of free list
    assert_eq!(*pq.free_ptr.get_mut(), Some(5));
    assert_next(&mut pq, 5, None);

    // ------

    // Check other pops for good measure
    let popped = pq.pop();
    assert_eq!(popped, Some(0));
    assert_eq!(pq.min(), Some(0));

    assert_eq!(*pq.head_ptr.get_mut(), Some(0));
    assert_eq!(*pq.min_ptr.get_mut(), Some(2));
    assert_tail(&mut pq, 4);
    // Verify edges of free list
    assert_eq!(*pq.free_ptr.get_mut(), Some(6));
    assert_next(&mut pq, 5, None);

    // ------

    let popped = pq.pop();
    assert_eq!(popped, Some(0));
    assert_eq!(pq.min(), Some(1));

    assert_eq!(*pq.head_ptr.get_mut(), Some(0));
    assert_eq!(*pq.min_ptr.get_mut(), Some(0));
    assert_tail(&mut pq, 4);
    // Verify edges of free list
    assert_eq!(*pq.free_ptr.get_mut(), Some(2));
    assert_next(&mut pq, 5, None);

    // ------

    // This here pops the head
    let popped = pq.pop();
    assert_eq!(popped, Some(1));
    assert_eq!(pq.min(), Some(2));

    assert_eq!(*pq.head_ptr.get_mut(), Some(1));
    assert_eq!(*pq.min_ptr.get_mut(), Some(1));
    assert_tail(&mut pq, 4);
    // Verify edges of free list
    assert_eq!(*pq.free_ptr.get_mut(), Some(0));
    assert_next(&mut pq, 5, None);
}

#[cfg(loom)]
#[cfg_attr(loom, test)]
fn concurrent_pop_middle() {
    loom::model(|| pop_middle());
}

#[cfg_attr(not(loom), test)]
fn reinsert() {
    let mut pq = PriorityQueue::<i32, 5>::new();

    // Arrange the queue such that it is empty, but node at index 0 is not the head of the free list
    pq.insert(100).unwrap();
    pq.insert(200).unwrap();
    pq.insert(300).unwrap();
    pq.pop();
    pq.pop();
    pq.pop();

    assert_eq!(*pq.head_ptr.get_mut(), None);
    assert_eq!(*pq.min_ptr.get_mut(), None);
    // Verify edges of free list
    assert_eq!(*pq.free_ptr.get_mut(), Some(2));
    assert_next(&mut pq, 4, None);

    // Now reinsert a value and see what happens
    pq.insert(200).unwrap();

    assert_eq!(*pq.head_ptr.get_mut(), Some(2));
    assert_eq!(*pq.min_ptr.get_mut(), Some(2));
    assert_tail(&mut pq, 2);

    pq.insert(100).unwrap();
    pq.insert(300).unwrap();

    assert_eq!(*pq.head_ptr.get_mut(), Some(2));
    assert_eq!(*pq.min_ptr.get_mut(), Some(1));
    assert_tail(&mut pq, 0);

    assert_eq!(pq.pop(), Some(100));
    assert_eq!(pq.pop(), Some(200));
    assert_eq!(pq.pop(), Some(300));

    assert_eq!(*pq.head_ptr.get_mut(), None);
    assert_eq!(*pq.tail_ptr.get_mut(), None);
    assert_eq!(*pq.min_ptr.get_mut(), None);
    assert_eq!(*pq.free_ptr.get_mut(), Some(0));
}

#[cfg(loom)]
#[cfg_attr(loom, test)]
fn concurrent_reinsert() {
    loom::model(|| reinsert());
}
