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
  title: [Work in Progress: A Concurrent Priority Queue with Constant-Time Blocking],

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
There exists many applications based on @PQ:pla, including real-time scheduling, event management,
and graph algorithms. In embedded and real-time systems, @DP scheduler kernel implementations
typically rely on @PQ:pla to manage incoming task arrivals and retrieve the highest priority task to
be executed. These data structures are challenging to implement correctly and efficiently in a
concurrent environment; they have therefore been an area of extensive research.

One of the main challenges of such algorithms is limiting the blocking time. Indeed, synchronizing
concurrent accesses to shared data structures often rely on mutual exclusion locks (_mutex_). On
single-core systems, these locks are typically implemented as critical sections--i.e., a section of
code which executes with interrupts disabled. However, schedulability criteria and task execution
jitter are generally dependent on the length of the _longest_ critical section in a given system; it
is therefore of interest to limit locks to a strict minimum.

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
node manipulations also based on @CAS operations. These operations are however fallible, and thus
unsuitable for hard real-time kernel implementations, as the worst case blocking time is unbounded.


In this paper we sketch a concurrent priority queue implementation, aiming for constant upper bounds
on blocking times targeting single core @COTS hardware.

== Background and Motivation -- @EDF:lo Scheduling
<sec:background>
@PQ:pla are the cornerstone of @EDF kernel implementations, a @DP scheduling paradigm. In common
priority queues, elements are allowed to be extracted under some given ordering. Classical
implementations include binary heaps, binomial heaps, Fibonacci heaps, and pairing heaps.

We consider an @EDF kernel where arriving tasks $J_i$ are each associated with two interrupt
handlers:
+ They are first signalled to an arrival handler $A_i$, which is assigned the maximum system
  priority. This handler captures the task's arrival timestamp `TS`, and may then either dispatch
  the task to run on a lower priority handler, or enqueue the task in a priority queue for later
  retrieval and execution (@fig:arrival-handler).
+ As tasks are dispatched on their dispatch handlers $D_i$, their payload is executed when the
  dispatch handler is executed by the interrupt controller. When the tasks completes, the dispatch
  handler runs a cleanup routine before returning. If the `min(PQ)` has an absolute deadline which
  is shorter than the next task to execute's deadline, then the highest priority task is extracted
  from `PQ` and dispatched (@fig:interrupt-handler).

Therefore, for the purpose of @EDF scheduling, we seek a priority queue implementation with the
following properties:

- Support for concurrent access from multiple execution contexts (e.g., threads or interrupts
  handlers).
- Bounded blocking times for concurrent access, with constant time $cal(O)(1)$ upper bounds.
- Implementation should not depend on dynamic memory allocations, and should be resource efficient
  in terms of both memory and CPU usage.

#figure(
  placement: auto,
  image("../build/figs/arrival_handler.pdf", width: 35%),
  caption: [Example implementation of an @EDF arrival handler $A_i$.],
)
<fig:arrival-handler>

#figure(
  placement: auto,
  image("../build/figs/interrupt.pdf", width: 70%),
  caption: [Arrival and dispatch handlers sorted by preemption level. Arrival handlers are assigned
    higher priorities to minimize time-stamp jitter.],
)
<fig:interrupt-handler>

#figure(
  placement: auto,
  image("../build/figs/extractMin.pdf", width: 100%),
  caption: [Extraction of the minimum element from the priority queue.],
)
<fig:extract-min>

// == Contributions

// #hl[Key contributions are [...]]

= In-place Priority Queue Approach

In the following we sketch the design and implementation of an in-place priority queue in Rust, and
discuss design decisions in regards to aforementioned requirements.

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

= Conclusions

== Future work

Opportunities to expand and improve this work are plentiful. In particular, we strive to:

- Elaborate a full implementation, as an open-source library in the Rust systems programming
  language;
- Provide a formal proof of correctness and memory safety;
- Integrate the @PQ implementation in an @EDF scheduler to evaluate its performance and real-time
  properties.











