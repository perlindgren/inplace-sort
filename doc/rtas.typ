#import "@preview/charged-ieee:0.1.4": ieee

#show: ieee.with(
  title: [TTA Scheduling in Rust for Safety-Critical Systems],

  abstract: [
    In this short paper, we sketch a thread-safe in-place priority queue implementation in the Rust systems level programming language. We extend the Rust critical section abstraction with support for preemption points, allowing us to give a $O(1)$ upper bound on blocking inferred in a concurrent setting. The overall complexity is $O(n)$ for `insert` and $O(1)$ for `new`, `peek` and `pop`, as expected for our in-place implementation. The queue is backed by a fixed-size array, and can be either statically, heap or stack allocated in compliance to the Rust ownership model.
  ],
  authors: (
    (
      name: "Anonymous Author",
      // department: [SRT],
      // organization: [Luleå University of Technology],
      // location: [Luleå, Sweden],
      // email: "per.lindgren@ltu.se",
    ),
  ),
  index-terms: ("Memory Safety", "Priority Queue", "Concurrency", "Blocking", "Defined Behavior", "Real-Time"),

  bibliography: bibliography("refs.bib"),
  figure-supplement: [Fig.],
)

= Introduction
Rust, with its strong emphasis on memory safety and concurrency, offers a promising platform for implementing safety, security, and timing critical systems. Priority queues find many applications in software systems, including real-time scheduling, event management, and graph algorithms. In this paper we will explore opportunities and challenges involved towards a thread-safe, in-place priority queue implementation. To provide upper bounds on blocking times in concurrent settings we extend Rust's critical section abstraction to support preemption points, allowing us to provide constant time $O(1)$ upper bounds on blocking times in concurrent settings.

== Background and Motivation <sec:background>

=== Rust for Critical Systems


=== Earliest Deadline First Scheduling

In common priority queues allow elements to be extracted under some given ordering. Classical implementations include binary heaps, binomial heaps, Fibonacci heaps, and pairing heaps.

For the purpose of Earliest Deadline First (EDF) scheduling, we seek a priority queue implementation with the following properties:

- Support for concurrent access from multiple execution contexts (e.g., threads or interrupts handlers).
- Bounded blocking times for concurrent access, ideally with constant time $O(1)$ upper bounds.
- Implementation should not depend on dynamic memory allocations, and should be resource efficient in terms of both memory and CPU usage.


== In-place Priority Queue Approach

In the following we sketch the design and implementation of an in-place priority queue in Rust, and discuss design decisions in regards to aforementioned requirements.

=== Data Structure

Listing <fig:pq_struct> shows the definition of our priority queue struct.
Backing storage (`data`) is provided by an array, which size is defined as a const generic parameter (`N`). The elements payload is defined as a `MaybeUninit<T>`, where `T` is the type of the items stored in the queue. The wrapping type allows us to leave the initial values uninitialized, and to move owned items in and out of the queue in a well defined manner. As such the `PriorityQueue` struct can be either statically, heap or stack allocated. For concurrent access however, we consider the case of a statically allocated queue unless else specified.

=== Operations

We consider the following set of operations:
- `new()`, to (statically) create a new empty priority queue,
- `insert(item: T)`, to concurrently insert an item,
- `peek()`, to concurrently retrieve the highest priority item without removing it, and
- `pop()`, to concurrently retrieve and remove the highest priority item.







#figure(
  // placement: none,
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
  caption: [Priority Queue struct definition. The queue is backed by a fixed-size array, and can be either statically, heap or stack allocated in compliance to the Rust ownership model.],
) <fig:pq_struct>


#figure(
  table(
    // Table styling is not mandated by the IEEE. Feel free to adjust these
    // settings and potentially move them into a set rule.
    columns: (auto, auto, auto),
    align: (auto, auto, auto),
    inset: (x: 8pt, y: 4pt),
    stroke: (x, y) => if y <= 1 { (top: 0.5pt) },
    //fill: (x, y) => if y > 0 and calc.rem(y, 2) == 0 { rgb("#efefef") },

    table.header([Task], [Period ms], [WCET ms]),

    [Task1], [40], [10],
    [Task2], [60], [15],
    [Task3], [80], [20],
  ),
  caption: [Example 1. System with three periodic tasks without resource sharing.],
  placement: none,
) <tab:example1>


// #figure(
//   placement: none,
//   image("../tta_ex1.drawio.svg"),
//   caption: [Example 1. TTA Scheduling example of three periodic tasks, non-preemptively scheduled under EDF.],
// ) <fig:tta_ex1>



$ R(t_i) = forall j, D(t_j)< D(t_i) sum C(t_j) + C(t_i) $ <eq:gamma>









