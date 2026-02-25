
#import "@preview/dashy-todo:0.1.3": todo
#import "../para-lipics/lib.typ": *
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

#let style-number(number) = text(gray)[#number]
#show raw.where(block: true): it => block(
  fill: luma(240),
  inset: 10pt,
  radius: 4pt,
  width: 100%,
)[
  #grid(columns: (1em, 1fr), align: (right, left), column-gutter: 0.7em, row-gutter: 0.6em, ..it
      .lines
      .enumerate()
      .map(((i, line)) => (style-number(i + 1), line))
      .flatten())
]

#let abstract = [
  In @DP scheduling, kernels generally rely on priority queues to select the task to be executed.
  The choice of queue implementation introduces tradeoffs with respect to software overhead, memory
  usage and blocking times. A key consideration is thread-safety and memory safety. In this paper,
  we propose an unsorted, thread-safe in-place priority queue allowing an $cal(O)(1)$ upper bound on
  inferred blocking, as well as $cal(O)(1)$ `insert`, $cal(O)(1)$ `min` and $cal(O)(N)$ `extractMin`
  operations. The queue is implemented as a linked list backed by a fixed-size array, and can be
  allocated either statically, on the heap or on the stack. Potential applications include real-time
  scheduling, event management, and graph algorithms where predictable and minimal blocking times
  are paramount. For the implementation we leverage on the strong typing and memory safety
  guarantees of the Rust systems level programming language. In order to obtain constant upper bound
  blocking we propose an extension to the `critical-section` crate, introducing structured and well
  defined preemption points and preemption regions within a critical section. Finally, we define a
  set of key invariants capturing sought properties and soundness of the priority queue, from which
  we argue the safety of the implementation.
]

#show: para-lipics.with(
  title: [A Concurrent Priority Queue with Constant-Time Blocking for EDF based hard Real-Time
    Scheduling],
  authors: (
    (
      name: "Per Lindgren",
      affiliations: "Luleå University of Technology, Sweden",
      email: "per.lindgren@ltu.se",
    ),
    (
      name: "Justin Beaurivage",
      affiliations: "Université du Québec à Trois-Rivières, Canada",
      orcid: "0009-0005-0452-1545",
      email: "justin.beaurivage@uqtr.ca",
    ),
    (
      name: "Valhe Kouneli",
      email: "valhe.kouneli@gmail.com",
      affiliations: "Tampere University, Finland",
    ),
  ),
  copyright: [ Per Lindgren, Justin Beaurivage, Valhe Kouneli],
  author-running: [P. Lindgren, J. Beaurivage and V. Kouneli],
  keywords: [memory safety, priority queue, concurrency, blocking, defined behavior, real-time, data
    structures, critical section],
  ccs-desc: [Computer systems organization $->$ Real-time systems; Computer systems organization
    $->$ Embedded software],
  anonymous: true,
  line-numbers: true,
  abstract: abstract,

  event-long-title: [38th Euromicro Conference on Real-Time Systems],
  event-acronym: "ECRTS",
  event-year: 2026,
  event-short-title: [ECRTS 2026],
  event-location: [Lund, Sweden],
  article-no: 99999,
)

#set math.equation(numbering: "(1)")

= Introduction
In embedded and real-time systems, @DP scheduler kernel implementations typically rely on @PQ:pla to manage incoming task arrivals and retrieve the highest priority task to be executed. These data structures are challenging to implement correctly and efficiently in a concurrent environment; they have therefore been an area of extensive research.

One of the main challenges of such algorithms is limiting the blocking time. Indeed, synchronizing concurrent accesses to shared data structures often rely on mutual exclusion locks (_mutex_). On single-core systems, these locks are typically implemented as critical sections where the lock-region executes with interrupts disabled. However, schedulability criteria and task execution jitter are generally dependent on the length of the _longest_ critical section in a given system; it is therefore of interest to limit worst-case lock duration to a strict minimum.

Some work has gone into implementing lock-free or concurrent @PQ:pla: the mound data structure  presented in @liuLockFreeArrayBasedPriority2011 achieves lock-free $cal(O)(log(log(N)))$ `insert` and $cal(O)(log(N))$ `extractMin` operations. This @PQ uses atomic @CAS operations which are assumed infallible; resource-limited embedded systems rarely implement truly infallible @CAS operations, such as is the case for the ubiquitous ARM Cortex-M family of @COTS microcontrollers @arm-v7m-arm. Other implementations use skip-lists and randomized access to amortize asymptotic time complexity @sundellFastLockfreeConcurrent2003. Some work has also gone into limiting a @PQ's I/O operations between an internal cache and external memory, while retaining a favorable amortized time complexity for its operations @brodalExternalMemoryPriorityQueues2025. Finally, while not a PQ, in @harrisPragmaticImplementationNonblocking2001, the authors propose a concurrent linked list, with node manipulations also based on @CAS operations. We however deem these approaches unsuitable for hard real-time kernel implementations targeting single-core @COTS hardware, as the worst case blocking time is unbounded when accounting for retried operations.

In this paper we propose a concurrent priority queue implementation leveraging Rust's strong typing and memory safety guarantees. Our approach is based on mutual-exclusion implemented as interrupt-free lock-regions, thus suitable for deployment on single-core @COTS hardware.

Key contributions of this work include:
- An in-place, array-based linked list priority queue implementation, with $cal(O)(1)$ `insert`, $cal(O)(1)$ `min` and $cal(O)(N)$ `extractMin` operations.
- An extension to the embedded Rust foundational `critical-section` crate, introducing structured preemption points and preemption regions within a critical section. For our proposal, we present safety argumentation and show compliance to rust ownership and borrowing rules.
- A set of key invariants capturing sought properties and soundness of the priority queue, from which we argue the safety and soundness of the implementation.
- Leveraging the proposed preemption point abstraction we show that worst case blocking time has a constant upper bound of $cal(O)(1)$, thus suitable for hard real-time scheduling applications.
- By introducing a work-stealing mechanism, the amortized complexity can maintain the $cal(O)(N)$ `extractMin` also for the current case.
- Applied to an @EDF scheduler, the proposed design allows for minimal task dispatch latency, free of priority inversion, and with minimal jitter.

= Background and Motivation -- @EDF:lo Scheduling
<sec:background>
@PQ:pla are a cornerstone of @EDF kernel implementations, a @DP scheduling paradigm. In common priority queues, elements are allowed to be extracted under some given ordering. Classical implementations include binary heaps, binomial heaps, Fibonacci heaps, and pairing heaps.

We consider an @EDF kernel where arriving tasks $J_i$ are each associated with two interrupt handlers:
+ They are first signalled to an arrival handler $A_i$. This handler captures the task's arrival timestamp `TS`, and may then either dispatch the task to run on a lower priority handler, or enqueue the task in a priority queue for later retrieval and execution (@fig:arrival-handler and @fig:interrupt-handler top).
+ As tasks are dispatched on their dispatch handlers $D_i$, their payload is executed when dispatch handler is executed by the interrupt controller. When the tasks completes, the dispatch handler take as scheduling decision. If `min(PQ)` has an absolute deadline which is shorter than the next task to execute's deadline, then the highest priority task is extracted from `extractMin(PQ)` and dispatched (@fig:interrupt-handler bottom).
+ The priority of arrival and dispatch handlers is determined according to relative task deadlines,where the group of arrival handlers (@fig:interrupt-handler top) are assigned higher priority than the group of dispatch handlers (@fig:interrupt-handler bottom), to minimize time-stamp jitter.

Therefore, for the purpose of @EDF scheduling, we seek a priority queue implementation with the
following properties:<sec:requirements>

- Support for concurrent access from multiple execution contexts (e.g., threads or interrupts handlers).
- Bounded blocking times for concurrent access, with constant time $cal(O)(1)$ upper bounds.
- Implementation should not depend on dynamic memory allocations, and should be resource efficient in terms of both memory and CPU usage.

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

The Rust Embedded Working Group develops and maintains a set of foundational libraries and tools for embedded development in Rust. Among these, the `critical-section` crate provides a generic _critical section_ abstraction. The crate defines a trait `Impl` to be implemented for each supported target architecture (@fig:rust-cs-trait). In context of single-core bare-metal embedded systems, the `acquire` and `release` functions associated to the `Impl` trait are typically implemented by disabling and enabling interrupts, respectively. By design, exactly one implementation of the `Impl` trait must be provided for a given application. This property is crucial for ensuring that the critical section abstraction is sound, as it prevents the user from accidentally mixing multiple (conflicting) implementations, which could/would lead to undefined behavior (@UB) in Rust. Any attempt to break the uniqueness property is rejected at compile time.

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
  caption: [`critical-section` `Impl` trait definition.],
) <fig:rust-cs-trait>

#figure(
  placement: none,
  ```rust
  critical_section::with(|cs| {
      // This code runs within a critical section.
  });
  ```,
  caption: [Minimal example.],
) <fig:rust-critical-section>

== Preemption Points and Regions

In this work we propose an extension to the `critical section` abstraction, to provide preemption regions within a critical section while maintaining the advantages of structured nesting.

In order to generalize the design, we reformulate the `Impl` trait definition as seen in @fig:rust-cs-trait. The new API gives as the ability to in a deterministic way enter and exit both critical sections and preemption regions (within critical sections). While a small change on the surface, this gives us the expressive power lacking in the original API.

#figure(
  placement: none,
  ```rust
  pub unsafe trait Impl {
      /// Should return the "cs enable" and "cs disable" restore states.
      unsafe fn get_states() -> (RawRestoreState, RawRestoreState);

      /// Should return the current state, later to be restored by `set_state`.
      unsafe fn get_state() -> RawRestoreState;

      /// Should set the current state to the raw_restore_state.
      unsafe fn set_state(raw_restore_state: RawRestoreState);
  }
  ```,
  caption: [Proposed `Impl` trait definition.],
) <fig:rust-cs-new-trait>

Based on the new `Impl` trait definition, we formulate the preemption region functionality along with a re-implementation of the original `with` function using the new API (@fig:rust-preemption).

#figure(
  placement: none,
  ```rust
  /// Execute closure `f` in a preemptive region inside a critical section.
  ///
  /// Nesting critical sections is allowed. The inner critical sections
  /// are mostly no-ops since they're already protected by the outer one.
  ///
  /// # Panics
  ///
  /// This function panics if the given closure `f` panics. In this case
  /// the preemption region is released before unwinding.
  #[inline]
  pub fn preemption_within<R>(_cs: &mut CriticalSection, f: impl FnOnce() -> R) -> R {
      // Helper for making sure `release` is called even if `f` panics.
      struct Guard {}

      impl Drop for Guard {
          #[inline(always)]
          fn drop(&mut self) {
              let disable = unsafe { get_states().1 };
              unsafe { set_state(disable) }
          }
      }

      let enable = unsafe { get_states().0 };
      unsafe { set_state(enable) };
      let _guard = Guard {};

      f()
  }

  /// Execute empty closure in a preemptive region inside a critical section.
  ///
  /// Allows pending interrupts/context switches to be handled.
  ///
  /// See [`preemption_within`] for additional information.
  #[inline]
  pub fn preemption_point_within<R>(cs: &mut CriticalSection) {
      preemption_within(cs, || {});
  }

  /// Execute closure `f` in a critical section.
  ///
  /// Nesting critical sections is allowed.
  ///
  /// # Panics
  ///
  /// This function panics if the given closure `f` panics. In this case
  /// the critical section is released before unwinding.
  #[inline]
  pub fn with<R>(f: impl FnOnce(CriticalSection) -> R) -> R {
      // Helper for making sure `release` is called even if `f` panics.
      struct Guard {
          state: RestoreState,
      }

      impl Drop for Guard {
          #[inline(always)]
          fn drop(&mut self) {
              unsafe { set_state(self.state) }
          }
      }

      let state = unsafe { get_state() };
      let _guard = Guard { state };
      let disable = unsafe { get_states().1 };
      unsafe { set_state(disable) }

      unsafe { f(CriticalSection::new()) }
  }

  ```,
  caption: [Preemptive Region implementation.],
) <fig:rust-preemption>



#figure(
  placement: none,
  ```rust
  pub struct Mutex<T> {
      data: core::cell::UnsafeCell<T>,
  }

  // This is not actually safe, but serves only as an illustration
  impl<T> Mutex<T> {
      pub const fn new(data: T) -> Self {
          Self {
              data: core::cell::UnsafeCell::new(data),
          }
      }

      pub fn with_ref<R>(&self, _cs: &CriticalSection, f: impl FnOnce(&T) -> R) -> R {
          // Access the protected data within the critical section
          let data = unsafe { &*self.data.get() };
          f(data)
      }

      pub fn with_ref_mut<R>(&self, _cs: &mut CriticalSection, f: impl FnOnce(&mut T) -> R) -> R {
          // Access the protected data within the critical section
          let data = unsafe { &mut *self.data.get() };
          f(data)
      }
  }

  unsafe impl<T> Sync for Mutex<T> {}
  ```,
  caption: [Proposed _Mutex_ implementation.
  ],
) <fig:rust-mutex>

@fig:rust-mutex, depicts the proposed _Mutex_ implementation. The design ensures compliance to the Rust borrow model: you may create multiple immutable references to the protected data, or a single mutable reference, but not both at the same time. This invariant is achieved through the method signatures, `&`/ `&mut CriticalSection` arguments to the `with_ref`/`with_ref_mut` methods respectively. While internally _unsafe_, the _Mutex_ API provides a safe abstraction. The `Sync` trait implementation allows the _Mutex_ to be statically allocated and shared across execution contexts, thus suitable for concurrent access from multiple threads or interrupt handlers.

The proposed design is fundamentally different from the standard library and the `critical_section` _Mutex_ implementations, which both acts as guard types without clearly identified delimiting structure. Instead our approach is closure based, which allows us fine grained control over the boundaries of critical sections and preemption regions. While, similar to `critical_section` crate's `Mutex`, our design adopts the `CriticalSection` token as proof of mutual exclusion - however, our design strengthens the semantics in such a way that tokens cannot be copied or cloned. Thanks to the uniqueness property we can leverage the Rust borrow checker to at compile time enforce adherence to Rust's aliasing rules.

#figure(
  placement: none,
  ```rust
  #![no_std]
  #![no_main]

  use core::panic;

  use cortex_m as _;
  use cortex_m_rt::entry;
  use panic_halt as _;

  use preemption::Mutex;
  static MY_VALUE: Mutex<i32> = Mutex::new(0);

  #[entry]
  fn main() -> ! {
      critical_section::with(|mut cs| {
          MY_VALUE.with_ref_mut(&mut cs, |data| {
              cortex_m::asm::nop();
              // Would error: cannot borrow `cs` as mutable more than once at a time
              // MY_VALUE.with_ref(&cs, |data| *data);
              *data += 1;
          });

          critical_section::preemption_within(&mut cs, || {
              cortex_m::asm::nop();
              // Would error: cannot borrow `cs` as immutable because it is also borrowed as mutable
              // MY_VALUE.read(&cs, |data| *data);
          });

          cortex_m::asm::bkpt();

          MY_VALUE.with_ref_mut(&mut cs, |data| *data += 2);

          let r = critical_section::with(|mut cs| {
              cortex_m::asm::nop();
              let r = MY_VALUE.with_ref(&cs, |data| *data);
              critical_section::preemption_within(&mut cs, || {
                  cortex_m::asm::nop();
                  // Would error: cannot borrow `cs` as immutable because it is also borrowed as mutable
                  // MY_VALUE.with_ref(&cs, |data| *data);
              });
              cortex_m::asm::bkpt();
              r
          });

          MY_VALUE.with_ref_mut(&mut cs, |data| *data = r);
          cortex_m::asm::bkpt();
          // cs // Would error: lifetime may not live long enough, thus cannot be be leaked
      });

      loop {}
  }
  ```,
  caption: [Example usage of the `preemptive_region` API.],
) <fig:rust-preemption-example>

The example demonstrates how to access protected data within a critical section, and how to execute code with preemption enabled while ensuring that the `CriticalSection` token is not accessible within the preemptive region. Attempting to access the `CS` token within the preemptive region results in a compile-time error, thus enforcing the safety properties of the API. Moreover, it shows that Rust ownership and aliasing rules are successfully enforced, the compiler will reject all `Would error:` cases.

In @fig:rust-objdump, we show the corresponding assembly code generated by the Rust compiler for this example. To facilitate readability, each section is delimited by a `bkpt` instruction, which serves as a breakpoint for debugging purposes. Similarly, `nop` instructions are injected to show the entrance point of inner sections.

#figure(
  placement: none,
  ```
  0800043c <cm_preempt::__cortex_m_rt_main::h7b4e7a516b59e2fb>:
   800043c: b580         	push	{r7, lr}
   800043e: 466f         	mov	r7, sp
   8000440: f240 0100    	movw	r1, #0x0
   8000444: f3ef 8c10    	mrs	r12, primask
   8000448: f2c2 0100    	movt	r1, #0x2000
   800044c: 2201         	movs	r2, #0x1
   800044e: f382 8810    	msr	primask, r2
   8000452: bf00         	nop
   8000454: 680b         	ldr	r3, [r1]
   8000456: f04f 0e00    	mov.w	lr, #0x0
   800045a: 3301         	adds	r3, #0x1
   800045c: 600b         	str	r3, [r1]
   800045e: f38e 8810    	msr	primask, lr
   8000462: bf00         	nop
   8000464: f382 8810    	msr	primask, r2
   8000468: be00         	bkpt	#0x0
   800046a: 6808         	ldr	r0, [r1]
   800046c: 3002         	adds	r0, #0x2
   800046e: 6008         	str	r0, [r1]
   8000470: f3ef 8010    	mrs	r0, primask
   8000474: f382 8810    	msr	primask, r2
   8000478: bf00         	nop
   800047a: 680b         	ldr	r3, [r1]
   800047c: f38e 8810    	msr	primask, lr
   8000480: bf00         	nop
   8000482: f382 8810    	msr	primask, r2
   8000486: be00         	bkpt	#0x0
   8000488: f380 8810    	msr	primask, r0
   800048c: 600b         	str	r3, [r1]
   800048e: be00         	bkpt	#0x0
   8000490: f38c 8810    	msr	primask, r12
   8000494: e7fe         	b	0x8000494 <cm_preempt::__cortex_m_rt_main::h7b4e7a516b59e2fb+0x58> @ imm = #-0x4
  ```,
  caption: [ARM v7em, disassembly of @fig:rust-preemption-example, showcasing the Rust zero-cost abstractions.],
) <fig:rust-objdump>

- The first `nop`, relates the case where we are accessing the _Mutex_ as mutable reference inside of a critical section. The compiler backend is hoisting the memory address calculation (register `r1`) outside of the critical section boundary (`msr	primask, r2 (r2 = 1)`).

- The second `nop` relates to the preemption region, where the critical section has been released (`msr	primask, lr (lr = 0)`), and later restored (`msr	primask, r2 (r2 = 1)`). Inside of this region, the `CS` token is inaccessible, any attempted access would lead to a compile-time error.

- The first `bkpt` instruction reached when we have returned into the critical section. At this point we can now access the `CS` token again, and thus the protected data. The third `nop` is reached after we have entered the critical section in a nested manner (e.g., calling into code with internal critical section).

- The fourth `nop` is reached inside of inner preemption region (once again without access to the `CS` token).

- Finally the second `bkpt` is reached when returning to the inner critical section and the third when returning to the outer critical section.

== Performance Evaluation

As seen in the above example, the compiler backend is able to cleverly re-use registers, matching/surpassing carefully hand-optimized assembly code. In particular, the critical section entry takes exactly 2 instructions, while critical section exit, preemptive region entry and exit take exactly 1 instruction each (this under the assumption that register pressure does not force stack spilling). In effect, our critical section implementation is truly zero-cost. In Rust terminology, implying that the abstracted API does not introduce any additional overhead compared to a carefully optimized manual implementation.

= In-place Priority Queue Approach

In the following we sketch the design and implementation of an in-place, concurrent priority queue, and discuss the design decisions in regard to the aforementioned requirements.

== Array-based Linked List

For the sake of simplicity, we implement the priority queue as a linked list backed by a fixed-size array (@fig:extract-min). In-place operations are achieved by maintaining a free list of available nodes.

- _insert_: Insertion is unsorted: elements are appended at the tail of the list. Node updates are protected by a critical section, which is implemented by disabling interrupts. This critical section is of constant time $cal(O)(1)$, as it only involves mutating a single node.

- _min_: At all times, the data structure maintains a record of its minimum element separately from the main linked list, allowing a $cal(O)(1)$ `min` operation. This record of the minimum element is updated at every list mutation (i.e., _insert_ and on _extractMin_), guaranteeing it remains synchronized with the main data structure.

- _extractMin_: Extraction of the minimum element is performed by traversing the list from head to tail to find the minimum element, and then removing it from the list. This operation has a time complexity of $cal(O)(N)$, where $N$ is the number of elements in the queue. However, since all insertions are performed exclusively and atomically--via a critical section--at the queue tail, inspecting all nodes guarantees that the minimum element of the list is found, since no node can be inserted at a location already traversed by the reader pointer. Moreover, critical sections can be limited to the length of inspecting or mutating a single node--and are thus constant-time ($cal(O)(1)$).

The implementation is thread-safe, thus allows for concurrent access from multiple execution contexts (the arrival and dispatch handlers, for the @EDF case under study).

== Work Stealing

Dispatch handlers execute concurrently, where a higher priority dispatch handler may preempt an ongoing _extractMin_ operation. The higher priority handler steals the read cursor including the reader pointer, the current minimum value encountered and a _previous pointer_---a pointer to the node previous to the current minimum value containing node. It then continues the traversal on behalf of the preempted _extractMin_ operation.

Once the traversal is complete, the minimum element, if any, is removed from the list, protected by a critical section. The critical section is of constant time $cal(O)(1)$, as it only involves a constant number of node updates. The stolen read cursor is set to indicate that the steal is complete, thus the resumed _extractMin_ can immediately return without additional traversal. This queue is therefore intended for single-core systems, where only a single task may execute at any given time, and it is therefore unnecessary to attempt to dispatch multiple tasks simultaneously.

The restart-free implementation ensures that the amortized work for _extractMin_ of each enqueued element is $cal(O)(N)$. In @sec:extractMin we will further elaborate on implementation specifics of the stealing mechanism, and argue that the amortized complexity remains $cal(O)(N)$ even in presence of preemptive execution among dispatch handlers.

== Safety Invariants<sec:safety_invariants>

Rust comes with strong safety guarantees, based on a strict type system, ownership and borrowing rules. However, in order to implement a concurrent priority queue, we need to occasionally opt-out of these guarantees using the `unsafe` keyword to manage shared mutable state. For the unsafe code, it is the responsibility of the developer to ensure soundness. In the following we will outline key safety
invariants for our implementation based on the below invariants:

Let $N$ be the set of (statically) allocated nodes, and $H, F, T$ denote the nodes specified by the head pointer, the free pointer, and the tail pointer, respectively. Let $italic("Cur")$ be the cursor used by _extractMin_, and if it's not empty, let $C$, and $italic(min)$,  $italic("prev")$ be the node specified the reader pointer, the minimum value encountered, and the node specified by the _previous pointer_, respectively. For $X in N$, denote ${X ->^* } =$#box[${n in N mid(|) exists space k in NN_0 : "next"^k (X) = n }$] and ${X ->^+ } = $#box[${n in N mid(|) exists space k in NN_+ : "next"^k (X) = n }$]. If $X$ is empty, both notations equal the empty set $emptyset$.

#math.equation(
  block: true,
  $N = {H ->^*} union {F ->^*} "and" {H ->^*} inter {F ->^*} = emptyset$,
)<eq:nodes>

#math.equation(block: true, $forall n in \{H ->^*\}, "initialized(n)"$)<eq:initialized>

#math.equation(block: true, $forall n in \{H ->^*\}, {F ->^*}: n in.not {n ->^+}$)<eq:no-loops>

#math.equation(
  block: true,
  $T "is not empty" => T in {H ->^*} "and" "next"(T) "is empty"$,
)<eq:tail_in_head>

#math.equation(
  block: true,
  $italic("Cur") "is not empty" => cases(
    C in {H -> *},
    italic(min) = min("value"(n) mid(|) n in {H ->^*} \\ {C ->^+}),
    italic("prev") "is empty and" italic(min) = "value"(H)\, "or" "value"("next"("prev")) = italic(min)
  )
  $
)<eq:cursor>

@eq:nodes stipulates that the set of initially allocated nodes is partitioned between the set of nodes reachable from the head pointer and the set of nodes reachable from the free pointer. As a corollary, we can infer that nodes reachable from $H$ head and $F$ free are in $N$, i.e., allocated. This invariant is crucial for ensuring that we never access memory outside of our allocated nodes, which would lead to @UB in Rust. Allocation/free and enqueue/dequeue operations are ensured to re-cycle the allocated nodes $N$.

@eq:initialized states that the set of nodes reachable from the head pointer are all initialized with a valid value according to the defined type. Rust requires that all values are initialized before they can be safely read. By upholding @eq:initialized, it is sufficient to show that values are always read through the head pointer to ensure that we satisfy Rust's safety guarantees and avoid @UB.

@eq:no-loops states that there are no loops in the lists starting at $T$ and $F$. This is essential to guarantee $cal(O)(n)$ for the _extractMin_ operation.

Assuming @eq:no-loops, @eq:tail_in_head stipulates that if the tail pointer $T$is not empty, it points to the *last* node in the list reachable from the head pointer $H$. This invariant is crucial for ensuring that we can safely assume that appended nodes are inserted at the tail of the list reachable from $H$.

Finally, @eq:cursor stipulates that the cursor is either empty, or it the assiated data has three qualities:
- the reader pointer points at some node reachable from the head pointer,
- the minimum value encountered is indeed the minimum value among nodes preceeding and including the last inspected node, and
- the _previous pointer_ points at the node before the node containing the minimum value encountered, or is empty if the minimum value is found at the head of the list.

For the implementation of the API operations, we have implemented allocation and insertion at index operations as private helper functions, assuming and ensuring invariants  @eq:nodes, @eq:initialized, @eq:no-loops, @eq:tail_in_head and @eq:cursor. The public API operations are implemented on top of these helper functions, and we argue that they uphold the safety invariants in a concurrent setting.

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

The safety invariants in @sec:safety_invariants are trivially upheld by the `new` function, as it initializes the `free` list to include all nodes, while the `head` and `tail` pointers are set to `None`, indicating an empty queue.

Blocking time is not a concern for the `new` function. In case of static allocation, the initialization is performed before `main` is executed, while in case of heap or stack allocation, the queue is not accessible until the `new` function returns, thus there is no concurrent access to the queue during initialization.


=== API: `insert(&mut self, value: T) -> Result<(), ()>`<sec:insert>

The `insert` operation (@fig:pq_insert) is responsible for adding a new value to the priority queue. The operation first checks if there is a free node available by checking the `free` pointer. If the queue is full (i.e., `free` is `None`), it returns a `QueueFull` error. Otherwise, it retrieves the index of the free node, initializes it with the new value, updates the `free` pointer to the next free node, and updates the linked list pointers accordingly. Invariants as follows:

The `insert` operation allocates (removes) a node $A$ from the free list ($F$), initializes it and inserts it at the tail ($T$) of the allocated list ($H$), honoring @eq:nodes and @eq:no-loops.
_Assuming_ $T$ indicates the tail of $H$, the new tail $T'$ is the allocated node $A$, thus @eq:tail_in_head holds. As we add an _initialized_ node $A$ to the set of _assumed_ initialized nodes reachable from $H$ the set of nodes reachable from $H$ remains initialized, thus @eq:initialized holds. 

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

Once the traversal is complete, if the cursor was stolen we return directly with a `None` value (as the element to dequeue has already been removed and handled at the preemption point. Else, we proceed to remove the minimum element and empty the cursor.

A new cursor will uphold @eq:cursor. A stolen cursor that upholds @eq:cursor will uphold it after each step of the travelsal. After the preemption point, a new node might be added at the tail, but it does not affect @eq:cursor. When the minimum node is removed, the cursor is emptied, maintaining the invariant.

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

#pagebreak()

== Example Execution

#figure(
  placement: none,
  image("../build/figs/operations_single_col.pdf", width: 100%),
  caption: [Example execution of the API operations.],
)
<fig:operations_single_col>

@fig:operations_single_col illustrates the state of the queue after a sequence of `insert` and `extractMin` operations. The queue is initially empty, and we insert three values (42, 1337, 38). We then perform three `extractMin` operations, which return the values in sorted order (38, 42, 1337), leaving the queue empty again.
#set enum(numbering: "a)")

+ in figure shows the initial state after `new`, where the queue is empty. All nodes are in the free list, and the head and tail pointers are `None`.

+ shows the state after `insert(42)`. This implies an allocation of node $A$ from the free list $F$, initialization of the allocated node $A$ with the value 42, and insertion at the tail $T$ of the list. The head $H$ and tail $T$ pointers are updated to point to the new node $A$.

+ shows the state after `insert(1337)`. This implies an allocation of node $A$ from the free list $F$, initialization of the allocated node $A$ with the value 1337, and insertion at the tail $T$ of the list. The tail pointer is updated to point to the new node $A$. The head pointer remains unchanged, as it still points to the first node containing 42.

+ shows the state after `insert(38)`. Implications follow previous example. The head pointer remains unchanged, as it still points to the first node containing 42. Any further `insert` operations would fail with a `QueueFull` error, as the free list $F$ is now empty.

+ shows the state after `extractMin()`. This implies that the node $A$ with the minimal value (38) is removed from the list headed by $H$ returned to the free list $F$. Node removal implies linking `cursor.min_index` (pointing to $A$), to the successor of $A$. In case $T$ and $A$ coincides ($A$ being the tail node), $T$ is set to `cursor.min_index` (predecessor of $A$). Finding the minimum element requires traversing the entire list, thus we cross a preemption point for each node traversed. Under preemption, the `extractMin` operation completes at the highest preemption level among the dispatch handlers, thus ensuring dispatch latency and jitter to be free of any priority inversion inferred by shared priority queue accesses.

+ shows the state after `extractMin()`. Follows the same implications as the previous `extractMin` operation, where the node $A$ with the minimal value (42) is removed from the list headed by $H$ and returned to the free list $F$.

+ shows the state after `extractMin()`. Follows the two previous examples. The node $A$ with the minimal value (1337) is removed from the list headed by $H$ and returned to the free list $F$. Here, both $H$ and $T$ refer to $A$. While $A$ is the last and only node, both $H$ and $T$ are updated to the predecessor of $A$ (`None`). At this point the queue is (again) empty, with all nodes returned to the free list $F$.


== Dispatcher Design

By performing the _extractMin_ operation at the level of the currently highest priority task, we ensure that the task dispatch latency is free of priority inversion, and the currently most urgent task isn't blocked by queue operations from lower priority dispatch handlers.

= Conclusions

In this paper we have outlined a concurrent priority queue implementation, and argued constant $cal(O)(1)$ blocking times for all operations. The in-place designs allows for efficient memory usage and static allocation, meeting our requirements for hard real-time scheduling applications. While priority queues using unsorted in-place array-based linked lists are well understood, the novelty here resides with the simplistic concurrent design, matching concrete requirements for hard-real time scheduling on single-core @COTS hardware. In the context of embedded hard real-time systems, the anticipated number of tasks is relatively small (often ranging from a hand-full to a few dozens), overhead of $cal(O)(N)$ for _extractMin_ is expected to be acceptable, while the constant time blocking times for all operations are expected to yield favorable scheduling performance.

For the implementation, we have revisited the Rust `critical section` abstraction, and proposed an extension to provide preemption regions within critical sections. The proposed design provides a safe APIs for the inherently unsafe operations (shared mutable state), at in Rust terms zero-cost. With the priority queue as an example, we have shown the $cal(O)(N)$ _extractMin_ operation can be be split into $N$ $cal(O)(1)$ operations, split at a well defined preemption point within the overarching critical section.

The `critical-section` crate, approaching 40 million downloads at the time of writing (Feb. 2026), is _foundational_ within the Rust embedded ecosystem. In its current form, the API provides a powerful abstraction for critical sections, but lacks the expressiveness to allow for preemption regions within critical sections. The proposed extension addresses this gap, providing a more flexible API that can be used to implement a wider range of concurrent data structures and algorithms, while still maintaining the safety guarantees of Rust.

== Future work

In future work, we plan to implement and evaluate the proposed design in a Stack Resource Policy @128747 based @EDF scheduler. For the implementation, we intend to characterize the blocking factors and overhead, and establish overhead aware response time and scheduling test. Furthermore, we aim to explore hardware-assisted interrupt time-stamping and study the practical effects of obtained jitter minimization to scheduling performance.

Regarding the `critical-section` crate extension, we plan to propose the design to the Rust embedded working group, and jointly work towards its inclusion in the foundational crate.

#bibliography("refs.bib")


