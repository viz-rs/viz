# Flowkit

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/viz-rs/viz#license)
[![Crates.io](https://img.shields.io/crates/v/flowkit.svg)](https://crates.io/crates/flowkit)
[![Downloads](https://img.shields.io/crates/d/flowkit.svg)](https://crates.io/crates/flowkit)
[![Docs](https://docs.rs/flowkit/badge.svg)](https://docs.rs/flowkit/latest/flowkit/)

A universal UI workflow library.

## Features

* **Path**: support multiple kinds of [`EdgeType`]s:

  - **SmoothStep**: smoothing and rounding corners by the [`squircle`]
  - **StraightStep**: straight steps
  - **Straight**: a straight line
  - **Curve**: a bezier curve

* **Simple**: easy to integrate into other frameworks

  - [bevy_flowkit]
  - [egui_flowkit]
  - [gpui_flowkit]
  - [makepad_flowkit]

* **SVG**: support SVG's path output

[`EdgeType`]: https://docs.rs/flowkit/latest/flowkit/edge/enum.EdgeType.html
[`squircle`]: https://www.figma.com/blog/desperately-seeking-squircles/

[bevy_flowkit]: https://docs.rs/bevy_flowkit
[egui_flowkit]: https://docs.rs/egui_flowkit
[gpui_flowkit]: https://docs.rs/gpui_flowkit
[makepad_flowkit]: https://docs.rs/makepad_flowkit
