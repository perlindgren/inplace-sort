
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
paper, we propose an unsorted, thread-safe in-place priority queue allowing an $cal(O)(1)$
upper bound on inferred blocking, as well as $cal(O)(1)$ `insert`, $cal(O)(1)$ `min` and
$cal(O)(N)$ `extractMin` operations. The queue is implemented as a linked list backed by a
fixed-size array, and can be allocated either statically, on the heap or on the stack. Potential
applications include real-time scheduling, event management, and graph algorithms where
predictable and minimal blocking times are paramount.

For the implementation we leverage on the strong typing and memory safety guarantees of the Rust systems level programming language. In order to obtain constant upper bound blocking we propose an extension to the `critical-section` crate, introducing structured and well defined preemption points and preemption regions within a critical section. Finally, we define a set of key invariants capturing sought properties and soundness of the priority queue, from which we argue the safety of the implementation.
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

In this paper we propose a concurrent priority queue implementation leveraging Rust's strong typing and memory safety guarantees. Our approach is based on mutual-exclusion implemented as interrupt-free lock-regions, thus suitable for deployment on single-core @COTS hardware.

Key contributions of this work include:
- An in-place, array-based linked list priority queue implementation, with $cal(O)(1)$ `insert`, $cal(O)(1)$ `min` and $cal(O)(N)$ `extractMin` operations.
- An extension to the embedded Rust foundational `critical-section` crate, introducing structured preemption points and preemption regions within a critical section. For our proposal, we present safety argumentation and show compliance to the `critical-section` crate's safety guarantees.
- A set of key invariants capturing sought properties and soundness of the priority queue, from which we argue the safety and soundness of the implementation.
- Leveraging the proposed preemption point abstraction we show that worst case blocking time has a constant upper bound of $cal(O)(1)$, thus suitable for hard real-time scheduling applications.
- By introducing a work-stealing mechanism, the amortized complexity can maintain the $cal(O)(N)$ `extractMin` also for the current case.
- Applied to an @EDF scheduler, the proposed design allows for minimal task dispatch latency, free of priority inversion, and with minimal jitter.

= Background and Motivation -- @EDF:lo Scheduling
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
following properties:<sec:requirements>

- Support for concurrent access from multiple execution contexts (e.g., threads or interrupts
  handlers).
- Bounded blocking times for concurrent access, with constant time $cal(O)(1)$ upper bounds.
- Implementation should not depend on dynamic memory allocations, and should be resource efficient
  in terms of both memory and CPU usage.

#figure(
  placement: auto,
  image("../build/figs/arrival_handler.pdf", width: 90%),
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

= Background Rust

Rust is a strongly-typed systems level programming language with a focus on safety and performance. Rust's ownership model and borrowing rules provide strong memory safety guarantees, preventing common issues such as null pointer dereferences, buffer overflows, etc. Rust ensures memory safety also in concurrent contexts, by means of the `Send` and `Sync` traits, where types implementing `Send` can be safely transferred across execution contexts, and types implementing `Sync` can be safely shared across execution contexts.

For our purpose, the priority queue is accessed from both arrival and dispatch handlers executing preemptively, thus the underlying data structure must implement the `Sync` trait.

== Rust Embedded Ecosystem

The Rust Embedded Working Group develops and maintains a set of foundational libraries and tools for embedded development in Rust. Among these, the `critical-section` crate provides a generic _critical section_ abstraction. The crate defines a trait `Impl` to be implemented for each supported target architecture (@fig:rust-cs-trait). In context of single-core bare-metal embedded systems, the `acquire` and `release` functions associated to the `Impl` trait are typically implemented by disabling and enabling interrupts, respectively. By design, exactly one implementation of the `Impl` trait must be provided for a given application. This property is crucial for ensuring that the critical section abstraction is sound, as it prevents the user from accidentally mixing multiple (conflicting) implementations, which could/would lead to undefined behavior (@UB) in Rust.  Applications breaking the uniqueness property are rejected at compile time.

Leveraging on Rust *zero-cost* abstractions, the `critical_section` crate defines a closure based API enforcing strict nesting to ensure that interrupt state is properly restored. The user provided closure takes a `CriticalSection` (`CS`) token argument - a zero-sized type that serves as a proof of code execution under protection of mutual exclusion. The `CS` token argument can neither be leaked outside of the critical section, nor created (forged) by user code.

@fig:rust-critical-section shows a minimal example. The `with` function is implemented in terms of the `Impl` trait associated functions `acquire` and `release`.

#figure(
  placement: none,
  ```rust
  pub unsafe trait Impl {
      // Required associated functions
      unsafe fn acquire() -> RawRestoreState;
      unsafe fn release(restore_state: RawRestoreState);
  }
  ```,
  caption: [Rust `Impl` trait definition for a critical section implementation.],
) <fig:rust-cs-trait>

#figure(
  placement: none,
  ```rust
  critical_section::with(|cs| {
    // This code runs within a critical section.
  });
  ```,
  caption: [Rust, minimal critical section example.],
) <fig:rust-critical-section>

== Preemption Points and Regions

In this work we propose an extension to the `critical section` abstraction, to provide preemption points and preemption regions within a critical section while maintaining the advantages of structured nesting. As a proof on concept we base the implementation (@fig:rust-preemption) on the current `critical_section` crate.


#figure(
  placement: none,
  ```rust
    mod preemptive_region {
      use critical_section::{CriticalSection, RestoreState};
      /// Executes a closure with preemption enabled inside a critical section.
      ///
      /// # Safety
      ///
      /// By requiring the CriticalSection (CS) token, we ensure that `with` can only
      /// be called from within a critical section.
      ///
      /// The CS token will be moved into the `with` function, but not leaked to into
      /// the closure `f`.
      ///
      /// The CS token will be returned to the caller after the closure `f` allowing
      /// reuse in consecutive calls to `with` within the same critical section.
      ///
      /// Given the assumption that RestoreState::invalid() represents a states
      /// where preemption is enabled, the closure `f` will thus execute:
      ///
      /// - Within a critical section.
      /// - With preemption enabled.
      /// - Without access to the CS token.
      ///
      pub fn with<R>(cs: CriticalSection, f: impl FnOnce() -> R) -> (R, CriticalSection) {
          unsafe { critical_section::release(RestoreState::invalid()) };

          let result = f();

          unsafe { critical_section::acquire() };
          (result, cs)
      }

      /// Create a well-defined preemption point within a critical section.
      ///
      /// # Safety
      ///
      /// See `with` for safety properties.
      ///
      pub fn point(cs: CriticalSection) -> CriticalSection {
          Self::with(cs, || {}).1
      }
  }
  ```,
  caption: [Preemptive Region implementation.],
) <fig:rust-preemption>

#figure(
  placement: none,
  ```rust
  mod private {
      use super::*;
      pub struct MyProtectedData {
          value: i32,
      }

      impl MyProtectedData {
          pub fn new(value: i32) -> Self {
              Self { value }
          }

          pub fn access(&self, cs: &CriticalSection) -> i32 {
              // Access the protected data within the critical section
              self.value
          }
      }
  }

  use private::MyProtectedData;
  fn main() {
      let protected_data = MyProtectedData::new(42);

      critical_section::with(|cs| {
          protected_data.access(&cs);

          let (_, cs) = preemptive_region::with(cs, || {
              // The CS token is unaccessible inside the closure
              // protected_data.access(&cs); <-- compile error, attempt to borrow moved value
          });

          let cs = preemptive_region::with(cs, || {
              // protected_data.access(&cs); <-- compile error, attempt te borrow moved value
          });

          //cs <-- compile error, value will not live long enough, thus cannot be be leaked
      });
  }
  ```,
  caption: [Example usage of the `preemptive_region` API. The example demonstrates how to access protected data within a critical section, and how to execute code with preemption enabled while ensuring that the `CriticalSection` token is not accessible within the preemptive region. Attempting to access the `CS` token within the preemptive region results in a compile-time error, thus enforcing the safety properties of the API.],
) <fig:rust-preemption-example>

== Native Support in the `critical-section` Crate

For the POC implementation we make the assumption that `RestoreState::invalid()` represents a state where preemption is enabled. Moreover, we rely on a fork of `critical-section`, where the `CriticalSection` type have been strengthened to enforce move semantics, crucial for preventing the `CS` token from being leaked into the preemptive region.

In future work, we plan to further investigate the `RestoreState` design and backing `Impl` trait definition to facilitate native support of preemptive regions in the upstreams `critical-section` crate.

= In-place Priority Queue Approach

In the following we sketch the design and implementation of an in-place, concurrent priority queue, and discuss the design decisions in regard to the aforementioned requirements.

== Array-based Linked List

For the sake of simplicity, we implement the priority queue as a linked list backed by a fixed-size array (@fig:extract-min). In-place operations are achieved by maintaining a free list of available nodes.

- _insert_: Insertion is unsorted: elements are appended at the tail of the list. Node updates are protected by a critical section, which is implemented by disabling interrupts. This critical section is of constant time $cal(O)(1)$, as it only involves mutating a single node.

- _min_: At all times, the data structure maintains a record of its minimum element separately from the main linked list, allowing a $cal(O)(1)$ `min` operation. This record of the minimum element is updated at every list mutation (i.e., _insert_ and on _extractMin_), guaranteeing it remains synchronized with the main data structure.

- _extractMin_: Extraction of the minimum element is performed by traversing the list from head to tail to find the minimum element, and then removing it from the list. This operation has a time complexity of $cal(O)(N)$, where $N$ is the number of elements in the queue. However, since all insertions are performed exclusively and atomically--via a critical section--at the queue tail, inspecting all nodes guarantees that the minimum element of the list is found, since no node can be inserted at a location already traversed by the reader pointer. Moreover, critical sections can be limited to the length of inspecting or mutating a single node--and are thus constant-time ($cal(O)(1)$).

The implementation is thread-safe, thus allows for concurrent access from multiple execution contexts (the arrival and dispatch handlers, for the @EDF case under study).

== Work Stealing

Dispatch handlers execute concurrently, where a higher priority dispatch handler may preempt an ongoing _extractMin_ operation. The higher priority handler steals the read cursor and the current minimum value encountered, continuing the traversal on behalf of the preempted _extractMin_ operation.

Once the traversal is complete, the minimum element, if any, is removed from the list, protected by a critical section. The critical section is of constant time $cal(O)(1)$, as it only involves a constant number of node updates. The stolen read cursor is set to indicate that the steal is complete, thus the resumed _extractMin_ can immediately return without additional traversal. This queue is therefore intended for single-core systems, where only a single task may execute at any given time, and it is therefore unnecessary to attempt to dispatch multiple tasks simultaneously.

The restart-free implementation ensures that the amortized work for _extractMin_ of each enqueued element is $cal(O)(N)$. In @sec:extractMin we will further elaborate on implementation specifics of the stealing mechanism, and argue that the amortized complexity remains $cal(O)(N)$ even in presence of preemptive execution among dispatch handlers.

== Safety Invariants<sec:safety_invariants>

Rust comes with strong safety guarantees, based on a strict type system, ownership and borrowing rules. However, in order to implement a concurrent priority queue, we need to occasionally opt-out of these guarantees using the `unsafe` keyword to manage shared mutable state. For the unsafe code, it is the responsibility of the developer to ensure soundness. In the following we will outline key safety
invariants for our implementation based on the below invariants:

Let $N$ be the set of (statically) allocated nodes, and $H, F, T$ denote the head pointer, free pointer, and tail pointer, respectively.

#math.equation(
  block: true,
  $N <--> \{H ->^*\} union \{F ->^*\}$,
)<eq:nodes>

#math.equation(block: true, $forall n in \{H ->^*\}, "initialized(n)"$)<eq:initialized>

#math.equation(
  block: true,
  $A in \{F ->^*\} and \{F ->^*\} space <--> space \{A\} union \{F' ->^*\}$,
)<eq:alloc_free>

#math.equation(
  block: true,
  $\{A\} union \{H' ->^*\} space <--> space \{H ->^*\} and A in \{H ->^*\}$,
)<eq:alloc_head>

#math.equation(
  block: true,
  $not (T ->^* emptyset) --> T == H ->^*$,
)<eq:tail_in_head>

@eq:nodes stipulates that the set of initially allocated nodes is partitioned between the set of nodes reachable from the head pointer ($H->^*$) and the set of nodes reachable from the free pointer ($H->^*$). As a corollary, we can infer that nodes reachable from $H$ head ($F$ free) are in $N$, i.e., allocated. This invariant is crucial for ensuring that we never access memory outside of our allocated nodes, which would lead to @UB in Rust.

In @eq:initialized, $H->^*$ denotes the set of nodes reachable from the head pointer. Rust requires that all values are initialized before they can be safely read. Therefore, the invariant stipulates that all nodes reachable from the head pointer are initialized with a valid value according to the defined type. This invariant is crucial for ensuring that we never read uninitialized memory, which would lead to undefined behavior (@UB) in Rust.

Thus by upholding @eq:initialized, it is sufficient to show that values are always read through the head pointer to ensure that we satisfy Rust's safety guarantees and avoid @UB.

@eq:alloc_free applies to allocation(free), right(left) implication, where $A$ denotes a node in the free list $F ->^*$, and $F' ->^*$ relates the state after(before) allocation(free). The invariant stipulates that $A$ is reachable from the free pointer before(after) the transition. This invariant is crucial for ensuring that we never access memory that has been deallocated, which would lead to @UB in Rust.
Analogously, @eq:alloc_head, cover enqueue(dequeue) of nodes reachable from the head pointer $H$. Together with @eq:nodes, allocation/free and enqueue/dequeue operations are ensured to re-cycle the allocated nodes $N$.

Finally, @eq:tail_in_head stipulates that if the tail pointer $T$is not empty, it points to the *last* node in the list reachable from the head pointer $H$. This invariant is crucial for ensuring that we can safely assume that appended nodes are inserted at the tail of the list reachable from $H$.

For the implementation of the API operations, we have implemented allocation and insertion at index operations as private helper functions, assuming and ensuring invariants @eq:initialized, @eq:alloc_free, @eq:alloc_head, and @eq:tail_in_head. The public API operations are implemented on top of these helper functions, and we argue that they uphold the safety invariants in a concurrent setting.

== Data Structure and API

=== Data Structure

#figure(
  placement: none,
  ```rust
  pub struct Cursor<T> {
      min_index: Option<u16>, // None, value indicates that index refers to head
      min_value: T,
      current_index: u16,
  }

  pub struct PriorityQueue<const N: usize, T: Debug + Copy + Clone + PartialOrd> {
    data: [MaybeUninit<T>; N],
    next: [Option<u16>; N],
    head: Option<u16>,
    tail: Option<u16>,
    free: Option<u16>,
    cursor: Option<Cursor<T>>,
  }
  ```,
  caption: [Priority Queue struct definition. The queue is backed by a constant sized array that can be either statically, heap or stack allocated in compliance to the Rust ownership model.],
) <fig:pq_struct>

The `PriorityQueue` struct is defined as shown in @fig:pq_struct. The size of the queue is determined as a compile-time constant `N`. The `data` field is an array of `MaybeUninit<T>`, which allows us to manage uninitialized memory safely. The `next` field is an array of `Option<u16>`, which represents the linked list structure of the queue. The `head`, `tail`, and `free` fields hold indices to the head and the tail of the queue, and the head of the free list, respectively. The `Option<u16>` enum type allows us to leverage the Rust type system to represent the absence of a next node (`None` variant), thus avoiding the need for sentinel values and their associated risks of @UB.
#footnote[This is just one possible implementation, alternatively we could pack the `data` and `next` fields into a single array of nodes, where each element is a struct containing both the value and the next pointer. However, we opted for the current design for its simplicity and clarity in illustrating the key concepts.]

=== API: `const fn new() -> Self`<sec:new>

Written entirely in safe Rust (implementation left out for brevity), the code implements the queue initialization, and is guaranteed to produce a valid `PriorityQueue` instance with all data elements in an uninitialized state, as seen in @fig:operations_single_col a). The `const fn`, allows for compile-time initialization, thus enabling static allocation of the queue.
#footnote[While only a subset of the Rust language is currently supported in _const context_, it proved sufficient for our implementation.]

The safety invariants @sec:safety_invariants are trivially upheld by the `new` function, as it initializes the `free` list to include all nodes, while the `head` and `tail` pointers are set to `None`, indicating an empty queue.

Blocking time is not a concern for the `new` function. In case of static allocation, the initialization is performed before `main` is executed, while in case of heap or stack allocation, the queue is not accessible until the `new` function returns, thus there is no concurrent access to the queue during initialization.


=== API: `insert(&mut self, value: T) -> Result<(), ()>`<sec:insert>

The `insert` operation (@fig:pq_insert) is responsible for adding a new value to the priority queue. The operation first checks if there is a free node available by checking the `free` pointer. If the queue is full (i.e., `free` is `None`), it returns a `QueueFull` error. Otherwise, it retrieves the index of the free node, initializes it with the new value, updates the `free` pointer to the next free node, and updates the linked list pointers accordingly. Invariants as follows:

The `insert` operation allocates (removes) a node $A$ from the free list ($F$), and inserts it at the tail ($T$) of the allocated list ($H$), along with with invariants @eq:alloc_free and @eq:alloc_head. The invariant @eq:nodes holds by @eq:alloc_free (alloc) and  @eq:alloc_head (enqueue)transitively. _Assuming_ $T$ indicates the tail of $H$, the new tail $T'$ is the allocated node $A$, thus @eq:tail_in_head holds. As we add an _initialized_ node $A$ to the set of _assumed_ initialized nodes reachable from $H$ the set of nodes reachable from $H$ remains initialized, thus @eq:initialized holds.

Manipulation of the priority queue is protected by a (global) critical section, thus safe. All operations are constant time $cal(O)(1)$.

#figure(
  placement: none,
  ```rust
   fn insert(&mut self, value: i32) -> Result<(), Error> {
       critical_section::with(|_cs| {
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
  ```,
  caption: [Priority Queue `insert` operation.],
) <fig:pq_insert>

=== API: `extractMin(&mut self) -> Option<T>`<sec:extractMin>

@fig:pq_extractMin illustrates in abbreviation the `extractMin` operation, which is responsible for removing and returning the minimum element from the priority queue. The operation traverses the linked list starting from the head, comparing each element to find the minimum value. Once the minimum element is found, it is removed from the list, and the linked list pointers are updated accordingly. The minimum value is then returned.

On function entry, we either steal an active cursor (and resume the traversal)or create a new cursor starting at the head of the list (code excluded for brevity).

During traversal, according to @eq:initialized reachable nodes headed by $H$ are initialized, thus `unsafe { self.data[next_index as usize].assume_init() }` is a safe operation. After the cursor has been updated, we introduce a synchronized preemption point.
On resume, if the cursor was stolen (indicated by a `None` value), we break the loop, else we continue the traversal.

Once the traversal is complete, if the cursor was stolen we return directly with a `None` value (as the element to dequeue has already been removed and handled at the preemption point. Else, we proceed to remove the minimum element, safety invariants can be argued  analogously with the `insert` operation and left out for brevity.

Regarding complexity, the algorithm is trivially $cal(O)(N)$, as we need to traverse the entire list to find the minimum element. In case of preemption by another dispatch handler, the stealing mechanism allows the higher priority handler to continue the traversal on behalf of the preempted handler. The highest priority dispatch handler will execute to completion, set the stolen cursor to `None`, dequeue and return the minimum element. Thus, we can conclude that each element is inspected exactly once, which leads us to conclude that the (amortized) complexity remains $cal(O)(N)$, even in presence of preemptive execution among dispatch handlers. As an effect, the `extractMin` will always complete at the highest preemption level among the dispatch handlers, thus ensuring dispatch latency and jitter to be free of any priority inversion inferred by shared priority queue accesses.

Moreover, the for each element traversed we cross a preemption point, ensuring that the blocking is bounded and constant time $cal(O)(1)$.

#figure(
  placement: none,
  ```rust
  pub fn extractMin(&mut self, mock_test: MockTest) -> Option<T> {
      CsSingleCore::with(|mut cs| {
          // steal or create new cursor
          // search minimal element in loop
          while let Some(next_index) = self.next[current_index as usize] {
              let next_value = unsafe { self.data[next_index as usize].assume_init() };
              if next_value < self.cursor.unwrap().min_value {
                  self.cursor = Some(Cursor {
                      min_value: next_value,
                      min_index: Some(current_index),
                      current_index: next_index,
                  });
              }
              self.cursor.replace(Cursor {
                  current_index: next_index,
                  ..self.cursor.unwrap()
              });

              CsSingleCore::preemption_point(&_cs);

              if let Some(cursor) = self.cursor {
                  current_index = cursor.current_index;
              } else {
                  break;
              }
          }
          // retire cursor (None value)
          // dequeue and return minimal element if any
      }
  }
  ```,
  caption: [Priority Queue `extractMin` operation.],
) <fig:pq_extractMin>







// #set enum(numbering: "a)")
// + in figure shows the initial state after `new`, where the queue is empty.
// + shows the state after `insert(42)`.
// + shows the state after `insert(1337)`.
// + shows the state after `insert(38)`.
// + shows the state after `extractMin()`.
// + shows the state after `extractMin()`.
// + shows the state after `extractMin()`. At this point the queue is empty again. At this point `min()` returns `None`, and `extractMin()` returns with an error.




#figure(
  placement: auto,
  image("../build/figs/operations_single_col.pdf", width: 100%),
  caption: [Example execution of the API operations. The figure illustrates the state of the queue after a sequence of `insert` and `extractMin` operations. The queue is initially empty, and we insert three values (42, 1337, 38). We then perform three `extractMin` operations, which return the values in sorted order (38, 42, 1337), leaving the queue empty again.],
)
<fig:operations_single_col>

// #figure(
//   placement: auto,
//   image("../build/figs/operations_two_col.pdf", width: 100%),
//   caption: [Extraction of the minimum element from the priority queue, with 3 concurrent readers and
//     a writer protected by a (global)critical section.],
// )
// <fig:operations_two_col>

== Dispatcher Design

By performing the _extractMin_ operation at the level of the currently highest priority task, we ensure that the task dispatch latency is free of priority inversion, and the currently most urgent task isn't blocked by queue operations from lower priority dispatch handlers.

= Conclusions

In this short paper we have sketched a concurrent priority queue implementation, and argued constant
time blocking times for all operations. The in-place designs allows for efficient memory usage and
static allocation, meeting our requirements for hard real-time scheduling applications. While priority queues using unsorted in-place array-based linked lists are well understood, the novelty here resides with the simplistic concurrent design, matching concrete requirements for hard-real time scheduling on single-core @COTS hardware. In the context of embedded hard real-time systems, the anticipated number of tasks is relatively small (often ranging from a hand-full to a few dozens), overhead of $cal(O)(N)$ for _extractMin_ is expected to be acceptable, while the constant time blocking times for all operations are expected to yield favorable scheduling performance.

== Future work

In future work, we plan to implement and evaluate the proposed design in a Stack Resource Policy @128747
based @EDF scheduler. For the implementation, we intend to leverage on the Rust language for
zero-cost abstractions, provide safe APIs for inherently unsafe operations, and characterize the blocking factors and overhead. Furthermore, we aim to explore hardware-assisted interrupt time-stamping and study the practical effects of obtained jitter minimization to scheduling performance.


#bibliography("refs.bib")


