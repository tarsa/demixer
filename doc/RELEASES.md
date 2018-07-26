# Releases

### Version 0.10.0

- Implemented bits runs trackers in tree nodes
  - three last bits runs lengths are kept, each capped to 1000
- FSM-based bit history states are no longer planned
  - FSM stands for Finite State Machine

### Version 0.9.0

- Implemented actual context mixing
  - main program now estimates compression ratio instead of printing bit
    histories

### Version 0.8.0

- Implemented probability refinement (APM/ SSE)
  - probabilities are stretched and squashed during that process
  - APM stands for AdaptiveProbabilityMap (structure used in this program)
  - SSE stands for Secondary Symbol Estimation (alternative name for APM)

### Version 0.7.0

- Added semi-stationary counters (probability estimators) to history sources

### Version 0.6.0

- Implemented entropy coding and estimation
  - entropy coding is not yet used
  - estimation is based on portable log2 implementation
  - portable log2 is based on fixed point numbers

### Version 0.5.0

- Implemented window sliding for tree based history source
  - reusing removed tree nodes is not implemented yet
  - cycling window buffer is also not implemented
  - these things will be done in subsequent releases

### Version 0.4.0

- Retrieving index of last occurrence of a context instead of first one

### Version 0.3.0

- Implemented retrieving index of first occurrence of a context when gathering
  histories

### Version 0.2.0

- Implemented bit history collection algorithms:
  - naive (brute force) which has minimal memory overhead but is very slow
  - fat hash maps which are still simple, but much faster than naive approach
    though very memory consuming
  - tree based which is complex, but fast and has reasonable memory requirements
  - all of them have to produce identical output - the non tree based ones exist
    mainly for correctness verification purposes

### Version 0.1.0

- Initial project setup
