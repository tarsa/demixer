# Backlog

- priority tasks
  - clean text model?
    - alphabet permutation is probably needed, but as a second step?
    - enabled when unfinished byte matches prefix
    - enabled on select bits, otherwise disabled and bits erased to 0
  - XML parsers and statistics
    - then XML modelling
  - wiki bbcode -> XML transformer
    - before that make an enwikX-only appendices that will turn enwiks into
      proper XMLs with proper bbcode if it's cut in half in a particular enwikX
  - pre-train estimators for indirect modelling
    - expect biggest boost on small files,
      but on bigger ones there should be too
  - move tests from inline tests modules to tests/ directory
  - hashed context simplified history source (like in PAQ series)
- add 48-bit WideDeceleratingEstimator
  - 32-bits for prediction
  - 16-bits for usage count
  - much more precise for very skewed probabilities
- address Clippy complaints
- add missing unit tests
  - InputWindow is missing ones
- implement low precision mode (chosen at compile time)
  - rename current fix::mul functions to mul_precise
  - always use mul_precise in LUT computations
  - rename stretch to stretch_precise
  - always use stretch_precise in squash LUT computations
  - precise squash is already fast enough
  - bench fast sigmoid and precise sigmoid
- use inclusive ranges ( `for i in 0..=5 { do_something(); }` )
- guard more functions with DO_CHECKS / NO_CHECKS
- replace FractOnly types with FractOr1 ones? UnitValue? UnitInterval?
  - UnitClose for unsigned
  - UnitOpen for signed
- improve test infrastructure
  - split test into multiple categories to save time on developing isolated
    features
- add support for different sizes of nodes pool and input window in tree based
  history source
  - keep in mind that next bit can potentially result in all active contexts
    triggering edge splits
  - therefore we need to ensure that before processing each input byte we have
    at least as many free nodes as there are active contexts on edges
  - when there are less free nodes then remove leftmost suffix until condition
    is satisfied
  - big input window can be especially beneficial when match model is added
  - match model has much smaller overhead than a full tree
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
    - simplify by relaxing types and using partial functions
      - just throw exceptions when non-matching message arrives
  - hardcoded schemes of data flow graphs for different thread configurations
    sounds plausible
