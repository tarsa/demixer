# Releases

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
