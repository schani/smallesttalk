# Generic Profiler Plan

## Why

The system is becoming large enough that performance problems can come from many layers:

- interpreter dispatch
- message sends and cache misses
- primitive execution
- source loading / compilation
- GUI rendering
- input/event handling
- specific tools like Browser / Workspace / Transcript

The profiler must therefore be **system-wide**, with tool-specific views layered on top.

## Goals

Build a profiler that can answer all of these questions:

- How much time is spent in the interpreter vs primitives?
- Which primitives are hottest?
- How many bytecodes and sends happen during an operation?
- Which GUI operations dominate rendering?
- Which tool or workflow triggered the work?
- What is the cost of a single frame, compile, or method execution?

## Architecture

## 1. Core VM profile data

Add a `VmProfile` structure to the Rust VM.

It should track at least:

- bytecodes executed
- sends performed
- super sends performed
- method cache hits
- method cache misses
- block activations
- primitive calls per primitive index
- time spent per primitive index
- total time in interpreted execution

Public API sketch:

- `vm.reset_profile()`
- `vm.profile_snapshot()`
- `vm.with_profiled_operation(name, ...)`

This is the foundation. Everything else builds on it.

## 2. Operation spans

Add lightweight named timing spans around major operations.

Examples:

- `compile_method_source`
- `compile_doit`
- `load_source`
- `run_method`
- `world render`
- `browser update`
- `png save`

Each span should record:

- call count
- total time
- average time
- worst time

This lets us correlate VM counters with user-visible work.

## 3. GUI/render profile data

Add a GUI-specific profile layer.

Track:

- rectangle fills
- form copies
- string draws
- glyphs drawn
- clip intersections
- clipped-out operations
- host display presents

This should be generic to the GUI framework, not specific to Browser.

## 4. Tool-level profile tags

Tools like Browser, Workspace, Transcript, Inspector, Debugger should annotate their work using named spans.

Examples:

- `browser model rebuild`
- `browser frame render`
- `workspace text redraw`
- `transcript append line`

This is optional metadata layered on top of the generic profiler.

## 5. Interactive diagnostics

Provide environment-variable-controlled live diagnostics.

Examples:

- `SMALLESTTALK_PROFILE=1`
- `SMALLESTTALK_PROFILE_VERBOSE=1`
- `SMALLESTTALK_BROWSER_MOUSE_DEBUG=1`

These should print rolling summaries such as:

- frame time
- VM counters delta for last frame
- top primitives by time
- current mouse state / event state

## 6. End-of-run summaries

When a profiled session exits, print a summary report:

- total runtime
- top spans by total time
- worst spans by max time
- total sends / bytecodes / primitive calls
- cache hit rate
- top primitives by time
- GUI render counts

## 7. Smalltalk-visible profiler later

Once the Rust-side data is stable, expose it to Smalltalk so tools can inspect profiling data in-image.

Possible future classes:

- `ProfilerSnapshot`
- `ProfilerSpan`
- `ProfilerPrimitiveStat`

This would allow an in-image profiler tool later.

## Implementation order

### Phase 1
- Add `VmProfile`
- Count bytecodes / sends / primitive calls
- Add primitive timing
- Add snapshot/reset API

### Phase 2
- Add generic named spans for high-level Rust operations
- Print text summaries on demand

### Phase 3
- Add GUI/render counters
- Add live diagnostics for interactive GUI sessions

### Phase 4
- Add tool-level spans
- Add Smalltalk-visible profiler objects

## First useful deliverable

The first implementation should be a **generic VM profiler**, not a browser-only profiler:

- bytecodes
- sends
- cache hits/misses
- primitive counts and timing
- optional live console output

Then Browser can become just one consumer of the profiler.
