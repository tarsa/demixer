# Versioning

Since this project is highly experimental by design it does not make sense to
strive for backwards compatibility. Consequently, it does not make sense to use
Semantic Versioning (SemVer) because by the rules the version would always stay
at 0.x.y (i.e. the major version would always be 0). Therefore the project uses
different versioning scheme.

### Versioning scheme

Scheme is X.Y.Z, where:
- X increases when a major feature is being exposed to users
- Y increases when a minor feature is added or changed and when a foundation for
  major feature is prepared
- Z increases on refactorings (since they can bring regressions), bug fixes and
  documentation or project structure improvements


### Major version goals

Here is a list of features that could cause major version (X) to increase:
- actual compression
- handling arbitrary length input (using sliding window trees)
- contexts with gaps and trees with symbols of variable length (not fixed to one
  byte)
- multithreading
- web based GUI
- cloud computing support
