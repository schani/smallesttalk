# Clipping Plan

## Problem

Ad-hoc widget-specific clipping is the wrong architecture.

A GUI framework must guarantee that drawing stays inside the visible region of a view without each tool reinventing clipping logic and without rendering every pane to an offscreen buffer first.

## Requirements

- clipping must be part of the GUI framework itself
- clipping must be inherited by child views
- widgets should draw normally and rely on the framework to enforce clipping
- clipping should work for fills, framed rectangles, text, and bitmap copy
- clipping should not require offscreen forms as an implementation strategy

## Design

### 1. Introduce `Canvas`

`Canvas` is the GUI drawing context. It owns:

- target `Form`
- current clip rectangle
- current origin / translation

Widgets draw on a `Canvas`, not directly on a raw `Form`.

### 2. Make clipping structural

Each `View` creates a clipped drawing context from its bounds:

- the incoming canvas already carries its parent's clip
- the view intersects that clip with its own bounds
- subviews inherit the resulting clipped canvas automatically

This makes clipping a property of the view tree.

### 3. Keep low-level drawing simple

The current low-level form primitives remain the same, but `Canvas` applies clipping before calling them.

Implemented operations:

- clipped rectangle fill
- clipped frame rectangle drawing
- clipped text drawing through the existing bitmap font renderer
- clipped form-to-form rectangle copy

### 4. System windows clip subviews to their content area

`SystemWindow` exposes a `contentBounds` rectangle. Child drawing is clipped to that area automatically, so tools like Browser, Transcript, and Workspace do not need custom clipping hacks.

### 5. Tools build on framework clipping

Browser panes, transcript text, workspace text, and labels should all rely on `Canvas` clipping instead of offscreen rendering.

## Implementation steps

1. Add `Canvas` to the kernel
2. Add rectangle intersection / translation helpers
3. Add `Form>>asCanvas` / `fullCanvas`
4. Refactor `View>>drawOn:` to operate on canvases and inherit clipping
5. Refactor `SystemWindow` so subviews are clipped to `contentBounds`
6. Refactor Browser / Transcript / Workspace to draw directly on clipped canvases
7. Validate with PNG snapshots and tests

## Future work

This gets clipping into the UI framework now.

Later improvements should include:

- local-coordinate subviews by using canvas translation more heavily
- a real `BitBlt` primitive with built-in clipping
- strike font glyph blitting with clipping
- invalidation / dirty region rendering
- scrollable views and text editors built on the same canvas model
