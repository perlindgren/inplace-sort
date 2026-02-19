
#import "@preview/bananote:0.1.2": *
#import "@preview/dashy-todo:0.1.3": todo
#import "@preview/subpar:0.2.2"
#import "@preview/abbr:0.3.0"

#show: abbr.show-rule
#abbr.load("abbrs.csv")
#abbr.config(style: it => text(it), space-char: sym.space)

#let hl(content) = {
  highlight[#content]
}

#set table(
  stroke: (x: none, y: none),
  // row-gutter: (1pt, auto),
  // column-gutter: (2mm, auto),
  inset: 0.4em,
)

#show figure.caption: set align(center)
#show table.cell.where(y: 0): set text(style: "normal", weight: "bold")

#show raw.where(block: true): set align(left)
#show raw.where(block: true): set text(size: 1em * 0.75)
#show figure.where(kind: raw): it => (
  block[
    #line(start: (0%, 0em), end: (100%, 0em), stroke: stroke(thickness: 0.5pt))
    #v(-1em)
    #it.body
    #line(start: (0%, -1em), end: (100%, -1em), stroke: stroke(thickness: 0.5pt))
    #v(-2em)
    #it.caption
  ]
)

#set math.equation(numbering: "(1)")


#show: note.with(
  title: [A Concurrent Priority Queue with Constant-Time Blocking for EDF based hard Real-Time Scheduling],
  authors: (
    ([Anonymous], []),
    //(name: "Anonymous"),
    // (
    //   name: "Anonymous authors for review",
    //   department: [Anonymous],
    //   organization: [Anonymous],
    //   location: [Anonymous],
    //   email: "anonymous@example.com",
    // ),
    // (
    //   name: "Anonymous authors for review",
    //   department: [Anonymous],
    //   organization: [Anonymous],
    //   location: [Anonymous],
    //   email: "anonymous@example.com",
    // ),
  ),
)

#abstract: [
In @DP scheduling, kernels generally rely on priority queues to select the task to be executed.
The choice of queue implementation introduces tradeoffs with respect to software overhead,
memory usage and blocking times. A key consideration is thread-safety and memory safety. In this
short paper, we sketch an unsorted, thread-safe in-place priority queue allowing an $cal(O)(1)$
upper bound on inferred blocking, as well as $cal(O)(1)$ `insert`, $cal(O)(1)$ `min` and
$cal(O)(N)$ `extractMin` operations. The queue is implemented as a linked list backed by a
fixed-size array, and can be allocated either statically, on the heap or on the stack. Potential
applications include real-time scheduling, event management, and graph algorithms where
predictable and minimal blocking times are paramount.
]

// index-terms: (
//   "memory safety",
//   "priority queue",
//   "concurrency",
//   "blocking",
//   "defined behavior",
//   "real-time",
//   "data structures",
//   "critical section",
// ),

// bibliography: bibliography("refs.bib"),
// figure-supplement: [Fig.],
// )

= Introduction
In embedded and real-time systems, @DP scheduler kernel implementations typically rely on @PQ:pla to
manage incoming task arrivals and retrieve the highest priority task to be executed. These data
structures are challenging to implement correctly and efficiently in a concurrent environment; they
have therefore been an area of extensive research.

One of the main challenges of such algorithms is limiting the blocking time. Indeed, synchronizing
concurrent accesses to shared data structures often rely on mutual exclusion locks (_mutex_). On
single-core systems, these locks are typically implemented as critical sections where the lock-region executes with interrupts disabled. However, schedulability criteria and task execution
jitter are generally dependent on the length of the _longest_ critical section in a given system; it
is therefore of interest to limit worst-case lock duration to a strict minimum.

Some work has gone into implementing lock-free or concurrent @PQ:pla: the mound data structure
presented in @liuLockFreeArrayBasedPriority2011 achieves lock-free $cal(O)(log(log(N)))$ `insert`
and $cal(O)(log(N))$ `extractMin` operations. This @PQ uses atomic @CAS operations which are assumed
infallible; resource-limited embedded systems rarely implement truly infallible @CAS operations,
such as is the case for the ubiquitous ARM Cortex-M family of @COTS microcontrollers @arm-v7m-arm.
Other implementations use skip-lists and randomized access to amortize asymptotic time complexity
@sundellFastLockfreeConcurrent2003. Some work has also gone into limiting a @PQ's I/O operations
between an internal cache and external memory, while retaining a favorable amortized time complexity
for its operations @brodalExternalMemoryPriorityQueues2025. Finally, while not a PQ, in
@harrisPragmaticImplementationNonblocking2001, the authors propose a concurrent linked list, with
node manipulations also based on @CAS operations. We however deem these approaches unsuitable for hard real-time kernel implementations targeting single-core @COTS hardware, as the worst case blocking time is unbounded when accounting for retried operations.


In this paper we sketch a concurrent priority queue implementation, aiming for constant upper bounds
on blocking times. Our approach is based on mutual-exclusion implemented as interrupt-free lock-regions, thus suitable for deployment on single-core @COTS hardware.

== Background and Motivation -- @EDF:lo Scheduling
<sec:background>
@PQ:pla are a cornerstone of @EDF kernel implementations, a @DP scheduling paradigm. In common
priority queues, elements are allowed to be extracted under some given ordering. Classical
implementations include binary heaps, binomial heaps, Fibonacci heaps, and pairing heaps.

We consider an @EDF kernel where arriving tasks $J_i$ are each associated with two interrupt
handlers:
+ They are first signalled to an arrival handler $A_i$. This handler captures the task's arrival timestamp `TS`, and may then either dispatch the task to run on a lower priority handler, or enqueue the task in a priority queue for later retrieval and execution (@fig:arrival-handler and @fig:interrupt-handler top).
+ As tasks are dispatched on their dispatch handlers $D_i$, their payload is executed when dispatch handler is executed by the interrupt controller. When the tasks completes, the dispatch handler take as scheduling decision. If `min(PQ)` has an absolute deadline which is shorter than the next task to execute's deadline, then the highest priority task is extracted from `extractMin(PQ)` and dispatched (@fig:interrupt-handler bottom).
+ The priority of arrival and dispatch handlers is determined according to relative task deadlines,where the group of arrival handlers (@fig:interrupt-handler top) are assigned higher priority than the group of dispatch handlers (@fig:interrupt-handler bottom), to minimize time-stamp jitter.

Therefore, for the purpose of @EDF scheduling, we seek a priority queue implementation with the
following properties:

- Support for concurrent access from multiple execution contexts (e.g., threads or interrupts
  handlers).
- Bounded blocking times for concurrent access, with constant time $cal(O)(1)$ upper bounds.
- Implementation should not depend on dynamic memory allocations, and should be resource efficient
  in terms of both memory and CPU usage.

#figure(
  placement: auto,
  image("../build/figs/arrival_handler.pdf", width: 80%),
  caption: [Example implementation of an @EDF arrival handler $A_i$.],
)
<fig:arrival-handler>

#figure(
  placement: auto,
  image("../build/figs/interrupt.pdf", width: 90%),
  caption: [Arrival and dispatch handlers sorted by preemption level. Arrival handlers are assigned
    higher priorities to minimize time-stamp jitter.],
)
<fig:interrupt-handler>

#figure(
  placement: auto,
  image("../build/figs/extractMin.pdf", width: 100%),
  caption: [Extraction of the minimum element from the priority queue, with 3 concurrent readers and
    a writer protected by a (global)critical section.],
)
<fig:extract-min>

// == Contributions

// #hl[Key contributions are [...]]

= In-place Priority Queue Approach

In the following we sketch the design and implementation of an in-place, concurrent priority queue,
and discuss the design decisions in regard to the aforementioned requirements.

== Array-based Linked List

For the sake of simplicity, we implement the priority queue as a linked list backed by a fixed-size
array (@fig:extract-min). In-place operations are achieved by maintaining a free list of available
nodes.

- _insert_: Insertion is unsorted: elements are appended at the tail of the list. Node updates are
  protected by a critical section, which is implemented by disabling interrupts. This critical
  section is of constant time $cal(O)(1)$, as it only involves mutating a single node.

- _min_: At all times, the data structure maintains a record of its minimum element separately from
  the main linked list, allowing a $cal(O)(1)$ `min` operation. This record of the minimum element
  is updated at every list mutation (i.e., _insert_ and on _extractMin_), guaranteeing it remains
  synchronized with the main data structure.

- _extractMin_: Extraction of the minimum element is performed by traversing the list from head to
  tail to find the minimum element, and then removing it from the list. This operation has a time
  complexity of $cal(O)(N)$, where $N$ is the number of elements in the queue. However, since all
  insertions are performed exclusively and atomically--via a critical section--at the queue tail,
  inspecting all nodes guarantees that the minimum element of the list is found, since no node can
  be inserted at a location already traversed by the reader pointer. Moreover, critical sections can
  be limited to the length of inspecting or mutating a single node--and are thus constant-time
  ($cal(O)(1)$).

The implementation is thread-safe, thus allows for concurrent access from multiple execution
contexts (the arrival and dispatch handlers, for the @EDF case under study).

== Work Stealing

Dispatch handlers execute concurrently, where a higher priority dispatch handler may preempt an
ongoing _extractMin_ operation. The higher priority handler steals the read cursor and the current
minimum value encountered, continuing the traversal on behalf of the preempted _extractMin_
operation.

Once the traversal is complete, the minimum element, if any, is removed from the list, protected by
a critical section. The critical section is of constant time $cal(O)(1)$, as it only involves a
constant number of node updates. The stolen read cursor is set to indicate that the steal is
complete, thus the resumed _extractMin_ can immediately return without additional traversal. This
queue is therefore intended for single-core systems, where only a single task may execute at any
given time, and it is therefore unnecessary to attempt to dispatch multiple tasks simultaneously.

The restart-free implementation ensures that the amortized work for _extractMin_ of each enqueued element is $cal(O)(N)$.

== Safety Invariants and API Operations

Rust comes with strong safety guarantees, based on a strict type system, ownership and borrowing rules. However, in order to implement a concurrent priority queue, we need to occasionally opt-out of these guarantees using the `unsafe` keyword to manage shared mutable state. For the unsafe code, it is the responsibility of the developer to ensure soundness. In the following we will outline key safety
invariants for our implementation based on the below invariants:

Let $N$ be the set of (statically) allocated nodes, and $H, F, T$ denote the head pointer, free pointer, and tail pointer, respectively.

#math.equation(
  block: true,
  $N <--> \{H ->^*\} union \{F ->^*\}$,
)<eq:nodes>

#math.equation(block: true, $H ->^* "initialized"$)<eq:initialized>

#math.equation(
  block: true,
  $A in F ->^*, \{H ->^*\} union \{F ->^*\} ->^* space <--> space \{A\} union \{H' ->^*\} union \{F' ->^*\}$,
)<eq:alloc>

#math.equation(
  block: true,
  $not (T -> emptyset) --> T == H ->^*$,
)<eq:tail_in_head>

@eq:nodes stipulates that the set of initially allocated nodes is partitioned between the set of nodes reachable from the head pointer ($H->^*$) and the set of nodes reachable from the free pointer ($H->^*$). As a corollary, we can infer that nodes reachable from $H$ head ($F$ free) are in $N$, i.e., allocated. This invariant is crucial for ensuring that we never access memory outside of our allocated nodes, which would lead to @UB in Rust.

In @eq:initialized, $H->^*$ denotes the set of nodes reachable from the head pointer. Rust requires that all values are initialized before they can be safely read. Therefore, the invariant stipulates that all nodes reachable from the head pointer are initialized with a valid value according to the defined type. This invariant is crucial for ensuring that we never read uninitialized memory, which would lead to undefined behavior (@UB) in Rust.

Thus by upholding @eq:initialized, it is sufficient to show that values are always read through the head pointer to ensure that we satisfy Rust's safety guarantees and avoid @UB.

@eq:alloc applies to allocation (deallocation), where $A$ denotes the allocated (deallocated) node, and $H'(F) ->^*$ relates the updated state. The invariants stipulate that the allocated node $A$ is reachable from the free pointer, and that the head and free pointers are updated accordingly to reflect the allocation (deallocation). This invariant is crucial for ensuring that we never access memory that has been deallocated, which would lead to @UB in Rust. Together with @eq:nodes, we allocation and deallocation operations are ensured to re-cycle the allocated nodes $N$.

Finally, @eq:tail_in_head stipulates that if the tail pointer is not empty, it points to the last node in the list reachable from the head pointer. This invariant is crucial for ensuring that we can safely append new nodes at the tail of the list.

For the implementation of the API operations, we have implemented allocation and insertion at index operations as private helper functions, assuming and ensuring invariants @eq:initialized, @eq:alloc and @eq:tail_in_head. The public API operations are implemented on top of these helper functions, and we argue that they uphold the safety invariants, thus ensuring that all API operations are safe to call in a concurrent context.


==== I) API operation: `const fn new() -> Self`

Written entirely in safe Rust, the code implements the queue initialization, and is guaranteed to
produce a valid `PriorityQueue` instance with all elements in an uninitialized state, as seen in
@fig:pq_new.

#figure(
  placement: none,
  // image("../figs/new.drawio.svg"), // hmm, bug..
  // image("../figs/new.jpg"), // hmm, bad..
  image("../figs/new.png"), // seems to work ok...
  caption: [Priority Queue initialization, ? indicates uninitialized elements.],
) <fig:pq_new>

==== II) API operation: `insert(&mut self, value: T) -> Result<(), ()>`

This is by far the most complex operation. We will cover it by covering the possible cases in a
non-concurrent context, and then discuss the concurrent case.

We have three main cases to consider:
1. If the queue is full, we return `Err(())`, and the queue remains unchanged. This is trivial and
  requires no unsafe code.
2. If the queue is empty, we insert the new value at the head of the queue, and update the `head`
  and `free` pointers accordingly. This is also straightforward and can be implemented in safe Rust.
  @fig:pq_first. depicts the state after inserting 4.

#figure(
  placement: none,
  image("../figs/first.png"), // seems to work ok...
  caption: [State of the queue after inserting 4.],
) <fig:pq_first>

3. If the queue is neither full nor empty, we need to find the correct position for the new value
  based on its priority, and insert it while maintaining the order. Reading an uninitialized value
  is illegal in Rust, as it implies @UB. However, following the `head`, always lead us to an
  initialized value (4 in our example). We start by introducing a local cursor variable, initialized
  to the `head` of the queue, and we read the value at the cursor.

  Now we have two cases to consider:

  a) the value to insert is of higher/equal priority, so insert before cursor, or b) the value to
  insert is of lower priority, so continue searching following the cursor. In case the cursor
  reaches the end of the queue, we insert at the end.

  The two cases are illustrated in @fig:pq_insert.

#figure(
  placement: none,
  image("../figs/insert.png"), // seems to work ok...
  caption: [State of the queue after: a) `insert(2)`, b) `insert(6)`. Notice here, higher priority
    implies smaller value.],
) <fig:pq_insert>

==== III) API operation: `fn peek(&self) -> Option<T>`

If the queue is empty, we return `None`, else we can safely read the `head` value due
@eq:initialized.

==== IV) API operation: `fn pop(&mut self) -> Option<T>`

If the queue is empty, we return `None`, and leave the queue unchanged. Else we can safely read the
`head` value due @eq:initialized, and we update the `head` pointer to the next node in the queue.
The popped node is then added to the free list, and we update the `free` pointer accordingly.

=== Concurrency and Blocking

So far we have covered the safety of the API operations in a non-concurrent context. Upholding the
invariant @eq:initialized is key to ensuring that we only read initialized values, @eq:alloc and
@eq:tail_in_head together ensures that nodes are re-cycled between the free list and the allocated list
in a well defined manner.

As mentioned in the background section, we can use the `critical-section` crate to provide mutual
exclusion for our API operations. However, for our implementation the `insert` operation would block
for $cal(O)(n)$ (insertion sort is linear time). While bounded, the excessive blocking is
undesirable in a real-time context. The problem can be somewhat mitigated by more efficient
implementations, e.g., the $cal(O)(k* log_2 n)$ binary heap. However, with the increased
implementation complexity the constant factor $k$ can be significant, and the blocking time can
still be excessive for real-time applications.

Instead we propose an extension to the critical section abstraction, where we can define preemption
points within the critical section. While not entirely *lock-free*, we can reduced the worst case
blocking time to a constant $cal(O)(1)$.

In @fig:operations_single_col cover the case of interest for arguing adherence to Rust safety invariants as well as assessment of blocking complexity.



#set enum(numbering: "a)")
+ in figure shows the initial state after `new`, where the queue is empty.
+ shows the state after `insert(42)`.
+ shows the state after `insert(1337)`.
+ shows the state after `insert(38)`.
+ shows the state after `extractMin()`.
+ shows the state after `extractMin()`.
+ shows the state after `extractMin()`. At this point the queue is empty again. At this point `min()` returns `None`, and `extractMin()` returns with an error.




#figure(
  placement: auto,
  image("../build/figs/operations_single_col.pdf", width: 100%),
  caption: [Extraction of the minimum element from the priority queue, with 3 concurrent readers and
    a writer protected by a (global)critical section.],
)
<fig:operations_single_col>

#figure(
  placement: auto,
  image("../build/figs/operations_two_col.pdf", width: 100%),
  caption: [Extraction of the minimum element from the priority queue, with 3 concurrent readers and
    a writer protected by a (global)critical section.],
)
<fig:operations_two_col>

== Dispatcher Design

By performing the _extractMin_ operation at the level of the currently highest priority task, we
ensure that the task dispatch latency is free of priority inversion, and the currently most urgent
task isn't blocked by queue operations from lower priority dispatch handlers.

= Conclusions

In this short paper we have sketched a concurrent priority queue implementation, and argued constant
time blocking times for all operations. The in-place designs allows for efficient memory usage and
static allocation, meeting our requirements for hard real-time scheduling applications. While priority queues using unsorted in-place array-based linked lists are well understood, the novelty here resides with the simplistic concurrent design, matching concrete requirements for hard-real time scheduling on single-core @COTS hardware. In the context of embedded hard real-time systems, the anticipated number of tasks is relatively small (often ranging from a hand-full to a few dozens), overhead of $cal(O)(N)$ for _extractMin_ is expected to be acceptable, while the constant time blocking times for all operations are expected to yield favorable scheduling performance.

== Future work

In future work, we plan to implement and evaluate the proposed design in a Stack Resource Policy @128747
based @EDF scheduler. For the implementation, we intend to leverage on the Rust language for
zero-cost abstractions, provide safe APIs for inherently unsafe operations, and characterize the blocking factors and overhead. Furthermore, we aim to explore hardware-assisted interrupt time-stamping and study the practical effects of obtained jitter minimization to scheduling performance.


#bibliography("refs.bib")


