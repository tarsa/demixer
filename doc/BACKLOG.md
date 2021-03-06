# Backlog

- refactor
  - split big files into smaller ones (like splitting mod.rs)
  - take into account visibility and mutability
  - remove Copy trait implementations where not strictly necessary
- improve test infrastructure
  - Rust's release mode disables numeric overflow and underflow checking
    - don't disable them in tests
  - split test into multiple categories to save time on developing isolated
    features
- implement window sliding by removing the leftmost suffices one by one
  - having a queue of leaves would be too memory consuming
  - instead we start finding a leaf corresponding to first prefix by descending
    straight from root
  - tree has limited depth so it will not degenerate performance heavily
  - one thing to check and possibly adjust is the longest active context
  - emulating node removal in naive and fat map history source implementations
    seems infeasible
  - instead keep and check as much invariants as possible
  - also keep the removal code as simple as possible
  - make verification tests that compares trees over identical windows but with
    different number of past dropped input symbols
  - such trees should have identical shape, have text indexes directly related
    (shifted by dropped input symbols number) but have possibly different edge
    counts and bit histories
  - implement window sliding in steps
    - step 1: removing longest prefixes one by one
    - step 2: reusing nodes (before that use over-provisioning)
    - step 3: cycling window buffer (before that use over-provisioning)
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
