# Backlog

- implement integrity checking for active contexts
- implement window sliding by removing the leftmost suffices one by one
  - having a queue of leaves would be too memory consuming
  - instead we start finding a leaf corresponding to first prefix by descending
    straight from root
  - tree has limited depth so it will not degenerate performance heavily
  - one thing to check and possibly adjust is the longest active context
  - first figure out how to emulate node removal in naive and fat map history
    source implementations
- add stationary counters to tree nodes (i.e. to the explicit, branching ones)
- bit histories should have 12-bits (as they have now) but be always based
  on rich FSM with state attributes like: rescaling_happened, capped_run_length, 
  no_branching, etc
  - wide bit histories can be then narrowed to specialized narrow bit histories
  - one narrow bit history is eg last (up to) 7 bits of history verbatim
  - another narrow bit history can be still skewed towards long runs but having
    smaller size (eg 8 bit) would be faster to adapt to (using stationary
    counters)
- implement multi-threading which will be used for encoder
  - thread based, without work stealing
    - every thread has a set of its responsibilities
    - every responsibility is assigned to exactly one thread
    - consider thread pinning, but as of now it's not in the stdlib
  - batch data into blocks to minimize synchronization overhead
    (but that prevents it from using to potentially parallelize decoding in 
    future)
  - use std::sync::mpsc for communication between threads
    - communication should work both ways, from producer to consumer and
      from consumer to producer
      - pair of channels per thread
      - recv side owned by thread listening to events (messages)
      - send side cloned to all cooperating threads (producers and consumers)
  - use mutexes for accessing shared batched data
    - wrap mutexes directly in messages?
  - CPU time statistics per thread
    - for Linux only (use conditional compilation)
    - libc crate doesn't have clock_gettime for Microsoft Windows OS
  - flexible generic assignment of responsibilities to threads may be very hard
    to implement (sounds like implementing materializers from Akka Streams)
  - hardcoded schemes of data flow graphs for different thread configurations
    sounds plausible
- add replaying HistorySource (taking recorded data from disk)
  - consider it when gathering histories dominates CPU time even when using
    multi-threading
  - special handling will be needed for stationary counters
  - stationary counters are dynamically initialized
  - therefore we would need to index each stationary counter in the tree
  - later the index must be mapped to actual counter using separate HashMap
    during both recording and replaying
  - when a node is created we must record the current run length along with
    new counter index
  - when a node is updated we only need to record the counter index
  - when a node is deleted we also need to record the counter index
  - alternatively we can hardcode stationary counters values in the recording
    - that would be way faster to replay
