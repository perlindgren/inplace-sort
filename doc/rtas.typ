#import "@preview/charged-ieee:0.1.4": ieee
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

#show: ieee.with(
  title: [Work in Progress: A Concurrent Priroity Queue with Constant-Time Blocking],

  abstract: [
    In @DP scheduling, kernels generally rely on priority queues to select the task to be executed.
    The choice of queue implementation introduces tradeoffs with respect to software overhead,
    memory usage and blocking times. A key consideration is thread-safety and memory safety. In this
    short paper, we sketch an unsorted, thread-safe in-place priority queue allowing an $cal(O)(1)$
    upper bound on inferred blocking, as well as $cal(O)(1)$ `insert`, $cal(O)(1)$ `min` and
    $cal(O)(N)$ `extractMin` operations. The queue is implemented as a linked list backed by a
    fixed-size array, and can be allocated either statically, on the heap or on the stack. Potential
    applications include real-time scheduling, event management, and graph algorithms where
    predictable and minimal blocking times are paramount.
  ],
  authors: (
    (
      name: "Anonymous authors for review",
      department: [Anonymous],
      organization: [Anonymous],
      location: [Anonymous],
      email: "anonymous@example.com",
    ),
    (
      name: "Anonymous authors for review",
      department: [Anonymous],
      organization: [Anonymous],
      location: [Anonymous],
      email: "anonymous@example.com",
    ),
  ),
  index-terms: (
    "memory safety",
    "priority queue",
    "concurrency",
    "blocking",
    "defined behavior",
    "real-time",
    "data structures",
    "critical section",
  ),

  bibliography: bibliography("refs.bib"),
  figure-supplement: [Fig.],
)

= Introduction
@PQ:pla find many applications in software systems, including real-time scheduling, event
management, and graph algorithms. In embedded and real-time systems, @DP scheduler kernel
implementations typically rely on @PQ:pla to store incoming tasks and retrieve the highest priority
task to be executed. These data structures are challenging to implement correctly and efficiently in
a concurrent environment; they have therefore been an area of extensive research.

One of the main challenges of such algorithms is limiting the blocking time. Indeed, synchronizing
concurrent accesses to shared data structures often rely on mutual exclusion locks (_mutex_). On
single-core systems, these locks are typically implemented as critical sections--i.e., a section of
code which executes with interrupts disabled. However, schedulability criteria and task execution
jitter are generally dependent on the length of the _longest_ critical section in a given system; it
is therefore of interest to limit locks to a strict minimum.

Some work has gone into implementing lock-free or concurrent @PQ:pla: the mound data structure
presented in @liuLockFreeArrayBasedPriority2011 achieves lock-free $cal(O)(log(log(N)))$ `insert`
and $cal(O)(log(N))$ `extractMin` operations. This @PQ uses atomic @CAS operations which are assumed
infaillible; resource-limited embedded systems rarely implement truly infaillible @CAS operations,
such as is the case for the ubiquitous ARM Cortex-M family of @COTS microcontrollers @arm-v7m-arm.
Other implementations use skip-lists and randomized access to amortize asymptotic time complexite
@sundellFastLockfreeConcurrent2003. While not a PQ, in
@harrisPragmaticImplementationNonblocking2001, the authors propose a concurrent linked list, with
node manipulations also based on @CAS operations. These operations are however faillible; making
such an implementation for a scheduler kernel, where retrying operations can lead to unbounded
execution time.

// Rust, with its strong emphasis on memory safety and concurrency, offers a promising platform for
// implementing safety, security, and timing critical systems.

In this paper we will explore opportunities and challenges involved towards a thread-safe, in-place
priority queue implementation. We strive to provide constant upper bounds on blocking times in
concurrent settings.



// === Rust for Critical Systems
// Rust adopts an ownership model, #todo[keep this section?]along with strict borrowing rules, to
// ensure memory safety without the need for a garbage collector. Rust supports bare-metal programming
// through its `#![no_std]` mode, excluding dependencies to the standard library and underlying
// operating system. This makes Rust a compelling choice for embedded and real-time systems, where
// predictability and low-level control are paramount.

// Rust's concurrency model is built around the concept of `Send` and `Sync` traits: types that
// implement `Send` can be transferred across thread boundaries, while types that implement `Sync` can
// be safely shared between threads.

// In this paper we sketch a thread-safe priority queue implementation, where we can have multiple
// threads inserting and removing items concurrently, which bounded blocking times.

// In a bare metal case, preemptive execution among interrupt handlers is commonly supported by
// underlying @COTS hardware (e.g., ARM Cortex-M). Resource protection is can be achieved through the
// official `critical-section` crate @critical_section, allowing a passed closure to be executed with
// interrupts disabled, and thus providing mutual exclusion.

// #figure(
//   placement: none,
//   ```rust
//   critical_section::with(|cs| {
//     // This code runs within a critical section.
//   });
//   ```,
//   caption: [The `with` function takes a closure that is executed under mutual exclusion. The `cs`
//     parameter is a token that represents the critical section, and can be used for race free access
//     to shared resources.],
// ) <fig:cs>

// While offering a structured and platform agnostic API, it lacks support for preemption points.


== Background and Motivation -- @EDF:lo Scheduling
<sec:background>
@PQ:pla are the cornerstone of @EDF kernel implementations, a @DP scheduling paradigm. In common
priority queues, elements are allowed to be extracted under some given ordering. Classical
implementations include binary heaps, binomial heaps, Fibonacci heaps, and pairing heaps.

We consider an @EDF kernel where arriving tasks are signalled to an interrupt handler, assigned to
the maximum system priority. This interrupt handler may then either dispatch the task to run on a
lower priority handler, or enqueue the task in a priority queue for later retrieval and execution
(@fig:arrival-handler). As tasks complete execution on their dispatch handlers, they may extract and
dispatch a new task from the priority queue, if the latter's absolute deadline is smaller than the
currently executing task. Therefore, for the purpose of @EDF scheduling, we seek a priority queue
implementation with the following properties:

- Support for concurrent access from multiple execution contexts (e.g., threads or interrupts
  handlers).
- Bounded blocking times for concurrent access, with constant time $cal(O)(1)$ upper bounds.
- Implementation should not depend on dynamic memory allocations, and should be resource efficient
  in terms of both memory and CPU usage.

#figure(
  placement: auto,
  image("../build/figs/arrival_handler.pdf", width: 30%),
  caption: [Example implementation of an @EDF arrival handler],
)
<fig:arrival-handler>

== Contributions

#hl[Key contributions are [...]]

= In-place Priority Queue Approach

In the following we sketch the design and implementation of an in-place priority queue in Rust, and
discuss design decisions in regards to aforementioned requirements.

=== Data Structure

#figure(
  placement: none,
  ```rust
  pub struct PriorityQueue<const N: usize, T>
  where
      T: Copy + Clone + PartialOrd,
  {
      data: [(MaybeUninit<T>, Option<u16>); N],
      head: Option<u16>,
      free: Option<u16>,
  }
  ```,
  caption: [Priority Queue struct definition. The queue is backed by a fixed-size array, and can be
    either statically, heap or stack allocated in compliance to the Rust ownership model.],
) <fig:pq_struct>

@fig:pq_struct shows the definition of our priority queue struct. Backing storage (`data`) is
provided by an array which size is defined as a const generic parameter (`N`). The elements payload
is defined as a `MaybeUninit<T>`, where `T` is the type of the items stored in the queue. The
wrapping `MaybeUninit` type is a union, allows us to leave the initial values uninitialized, and to
move owned items in and out of the queue in a well defined manner. As such the `PriorityQueue`
struct can be either statically, heap or stack allocated. For concurrent access however, we consider
the case of a statically allocated queue unless else specified.

=== API
We consider the following set of operations:
- `const fn new() -> Self`, const context queue constructor,
- `fn insert(&mut self, value: T) -> Result<(), ()>`, concurrent fallible insertion ,
- `fn peek(&self) -> Option<T>`, concurrent retrieval of the highest priority item without removing
  it, and
- `fn pop(&mut self) -> Option<T>`, concurrent retrieval and removal of the highest priority item.

=== Safety
Rust comes with strong safety guarantees, based on strict ownership and borrowing rules. However, in
order to implement a concurrent priority queue, we need to occasionally opt-out of these guarantees
using the `unsafe` keyword to manage shared mutable state. For the unsafe code, it is the
responsibility of the developer to ensure soundness. In the following we will outline key safety
invariants for our implementation based on the below invariants:

#math.equation(block: true, $H ->^* "initialized"$)<eq:initialized>

#math.equation(
  block: true,
  $A in F ->^*, H ->^* union F ->^* == \{A\} union H' ->^* union F' ->^*$,
)<eq:alloc>

#math.equation(
  block: true,
  $\{A\} union H ->^* union F ->^* == H' ->^* union F' ->^*, A in F' ->^*$,
)<eq:dealloc>

In @eq:initialized $H(F) ->^*$ denotes the set of nodes reachable from the `head`(`free`)
respectively. @eq:alloc applies to allocation specifically, where $A$ denotes a newly allocated
node, and $H'(F) ->^*$ relates the updated state. @eq:dealloc applies to deallocation specifically,
where $A$ denotes a newly deallocated node, and $H'(F) ->^*$ relates the updated state.

For the implementation of the API operations, we have implemented allocation and insertion at index
operations as private helper functions, assuming and ensuring invariants @eq:initialized, @eq:alloc
and @eq:dealloc.


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
@eq:dealloc together ensures that nodes are re-cycled between the free list and the allocated list
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

=== Preemption Point trait

@fig:preemption_point sketch the proposed `PreemptionPoint` trait. By requiring the `CsToken` we
ensure that the `preemption_point` function can only be called within a critical section. Thus the
implementation can safely, exit with interrupts disabled, on return.

#figure(
  placement: none,
  ```rust
  trait PreemptionPoint: CriticalSection {
      fn preemption_point(cs: &CsToken);
  }

  impl PreemptionPoint for cs_single_core {
      #[inline(always)]
      fn preemption_point(_cs: &CsToken) {
          enable_interrupts();
          // Allow preemption here
          disable_interrupts();
      }
  }
  ```,
  caption: [We extend on the critical section abstraction, by defining a `PreemptionPoint` trait, as
    a sub-trait of `CriticalSection`. Compiler barriers are enforced by the low-level interrupt
    enable/disable calls.],
) <fig:preemption_point>

=== Preemption Point for the `insert` Operation

Focusing on the `insert` operation, we can define a preemption point after each iteration of the
search loop, as illustrated in @fig:pq_insert_preemption. This allows other threads to access the
queue between iterations, and thus reduces the worst case blocking time to $cal(O)(1)$.

#figure(
  placement: none,
  ```rust
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
          cs_single_core::preemption_point(&cs); // preemption point
      }
  }
  ```,
  caption: [Insert operation with added preemption point.],
) <fig:pq_insert_preemption>

=== Thread safety

To argue thread safety of this approach we consider the following cases:

1. Preemption has pop:ed node(s) from the head of the queue.
2. Preemption has inserted node(s) before the cursor.
3. Preemption has inserted node(s) after the cursor.

= Conclusions

// #figure(
//   table(
//     // Table styling is not mandated by the IEEE. Feel free to adjust these
//     // settings and potentially move them into a set rule.
//     columns: (auto, auto, auto),
//     align: (auto, auto, auto),
//     inset: (x: 8pt, y: 4pt),
//     stroke: (x, y) => if y <= 1 { (top: 0.5pt) },
//     //fill: (x, y) => if y > 0 and calc.rem(y, 2) == 0 { rgb("#efefef") },

//     table.header([Task], [Period ms], [WCET ms]),

//     [Task1], [40], [10],
//     [Task2], [60], [15],
//     [Task3], [80], [20],
//   ),
//   caption: [Example 1. System with three periodic tasks without resource sharing.],
//   placement: none,
// ) <tab:example1>


// #figure(
//   placement: none,
//   image("../tta_ex1.drawio.svg"),
//   caption: [Example 1. TTA Scheduling example of three periodic tasks, non-preemptively scheduled under EDF.],
// ) <fig:tta_ex1>












