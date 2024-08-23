# h264bsd-rs

High-level bindings for [h264bsd](https://github.com/oneam/h264bsd) which is a decoder extracted from the Android Project.

It currently supports three possible image output formats through [av-data](https://crates.io/crates/av-data)'s frames:
- YUV
- RGB
- RGBA

Current quirks (the reasons why it's not <= 1.0 yet):
- `invalid memory reference` when I try to free the allocated data of pictures in DPB's buffer so it doesn't free (e.g don't use it if you plan on initializing the decoder many times during one run, otherwise it's not a problem as the OS will free the memory by itself)
- (to be updated with new quirks that I forgor about :skull:)