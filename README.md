# h264bsd-rs

High-level bindings for [h264bsd](https://github.com/oneam/h264bsd) which is a decoder extracted from the Android Project.

It currently supports three possible image output formats through [av-data](https://crates.io/crates/av-data)'s frames:
- YUV
- RGB
- RGBA