# Releases

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