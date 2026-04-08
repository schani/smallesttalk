# GUI Roadmap

This is the concrete execution breakdown for the Smalltalk GUI described in `GUI.md`.

Goal:

- build a **fully Smalltalk-implemented desktop GUI**
- use a **bitmap display** and **strike/bitmap fonts**
- use the **original Smalltalk font** as the initial system font if we can extract/import it
- deliver a live environment with:
  - Transcript
  - Workspace
  - Inspector
  - Browser
  - Debugger

The roadmap is written as implementation milestones with clear artifacts, dependencies, and demo targets.

---

## 0. Guiding rules

These rules should constrain every implementation step.

1. **Only the host boundary lives in Rust**
   - native window
   - framebuffer present
   - event pump
   - time
   - low-level bitmap operations

2. **Everything above that lives in Smalltalk**
   - forms
   - fonts
   - world
   - windows
   - widgets
   - tools

3. **Every milestone must produce something visible or usable**
   We do not want a huge invisible framework with no running result.

4. **Text is a first-class concern**
   Since the end goal is a browser/workspace environment, text drawing and editing are on the critical path.

5. **Use the historical Smalltalk look intentionally**
   The original strike font and monochrome/bitmap-first rendering should shape the system from the beginning.

---

## 1. Top-level milestone graph

### M0 — VM support for display and events
Deliver the host-facing primitives.

### M1 — Smalltalk graphics kernel
Forms, rectangles, points, display object, basic blit/fill/frame.

### M2 — Strike font import and text rendering
Import original Smalltalk font and render strings.

### M3 — World loop and event dispatch
Mouse/keyboard events, invalidation, damage repair, hand/cursor.

### M4 — Windows and foundational widgets
Windows, labels, buttons, lists, scroll bars, menus, split panes.

### M5 — Transcript and Workspace
First productive GUI tools.

### M6 — Inspector and Browser
Core development environment.

### M7 — Debugger and integrated live development
Full Smalltalk desktop loop.

### M8 — Polish and expansion
Color, icons, package tools, UI builder, etc.

---

## 2. Milestone details

## M0 — VM support for display and events

### Purpose
Create the smallest Rust substrate that lets Smalltalk own the GUI.

### Rust deliverables

#### Display/window primitives
- create host window
- query display size
- resize notification
- present framebuffer to host window

#### Event primitives
- fetch next event
- support event kinds:
  - mouse move
  - mouse button down/up
  - key down/up
  - window resize
  - quit

#### Time primitives
- monotonic millisecond clock
- sleep/wait for event timeout

#### Bitmap primitives
At least:
- bit block copy
- fill rectangle
- invert rectangle
- maybe draw line
- maybe scroll rectangle

### Smalltalk-facing abstraction target
The Rust layer should be usable via classes such as:
- `HostDisplay`
- `HostEventSource`
- `HostTime`

These may later be wrapped by richer Smalltalk objects like `Display` and `InputSensor`.

### Demo target
A Smalltalk script opens a host window and fills the framebuffer with a test pattern.

### Tests
- open/close window smoke test
- framebuffer present smoke test
- event queue smoke test
- deterministic bitmap primitive unit tests where possible

---

## M1 — Smalltalk graphics kernel

### Purpose
Build the graphics model inside Smalltalk.

### Smalltalk classes

#### Geometry
- `Point`
- `Rectangle`

#### Bitmap model
- `Form`
- `Display`
- `Cursor`
- `BitBlt`
- `DamageRecorder`

### Required protocols

#### `Point`
- `x`, `y`
- `+`, `-`
- comparisons
- conversions

#### `Rectangle`
- origin/corner or left/top/right/bottom
- `containsPoint:`
- `intersects:`
- `merge:`
- `insetBy:`
- `width`, `height`

#### `Form`
- dimensions/depth/bits
- `at:` / pixel access if needed for debugging
- `fill:`
- `copyBits:`
- `frameRect:`

#### `Display`
- singleton/current display
- `extent`
- `present`
- `fillWhite` / `fillBlack` / clear methods

#### `BitBlt`
- source form
- destination form
- source rect
- dest point
- clipping rect
- combination rule later if needed

### Demo target
Smalltalk draws:
- checkerboard
- framed rectangles
- simple cursor box
- offscreen form copied onto display

### Exit criteria
- Smalltalk can completely clear and redraw the screen each frame
- Smalltalk can maintain offscreen forms and copy them to display

---

## M2 — Strike font import and text rendering

### Purpose
Get real text on screen using the original Smalltalk font.

### Font workstreams

#### 2.1 Historical font acquisition
Need to locate/extract:
- original Smalltalk strike font data
- format details from image/font representation
- one or more canonical font sizes (at least the default system font)

#### 2.2 Conversion/import tooling
Need one conversion path from historical data into our runtime format.

Possible path:
- Rust extractor from historical Smalltalk font asset/image dump
- emit a simple neutral format, e.g.:
  - glyph bitmap table
  - widths
  - ascent/descent
  - baseline
  - character map

#### 2.3 Runtime font classes
- `StrikeFont`
- `TextStyle`
- maybe `GlyphCache`

### Required runtime behavior
- draw string at point
- compute string width
- line height
- ascent/descent
- missing-glyph fallback

### Early simplifications
- ASCII only first
- no kerning
- no international shaping
- monochrome glyph blits only

### Demo target
Show on screen:
- sample alphabet
- transcript-like scrolling lines
- window title text
- menu text

### Exit criteria
- `Display drawString:at:font:` works
- original Smalltalk font is loaded and used as system font

---

## M3 — World loop and event dispatch

### Purpose
Create the live GUI runtime.

### Smalltalk classes
- `InputSensor`
- `EventQueue`
- `Event`
- `MouseEvent`
- `KeyboardEvent`
- `World`
- `Hand`

### Responsibilities

#### `InputSensor`
- poll host events
- turn host structures into Smalltalk objects

#### `World`
- maintain root view/window list
- run main loop
- track dirty areas
- request redraws

#### `Hand`
- mouse position
- pressed buttons
- dragging target
- cursor state

### Main loop shape
Roughly:
1. poll or wait for events
2. convert to Smalltalk event objects
3. dispatch to hand/world/widgets
4. run pending step/update actions
5. redraw damaged regions
6. present display

### Demo target
- visible pointer movement
- click interaction with a test widget
- drag a rectangle in the world

### Exit criteria
- event pump fully controlled in Smalltalk
- damage-based redraw working

---

## M4 — Windows and foundational widgets

### Purpose
Build the first actual GUI framework.

### Class set
- `View`
- `CompositeView`
- `SystemWindow`
- `LabelView`
- `ButtonView`
- `ListView`
- `ScrollBarView`
- `MenuView`
- `SplitView`

### Base `View` protocol
- `bounds`
- `bounds:`
- `owner`
- `subviews`
- `addSubview:`
- `removeSubview:`
- `drawOn:`
- `handleEvent:`
- `hitTest:`
- `invalidate`
- `layoutSubviews`
- `wantsKeyboardFocus`

### Window behavior
- title bar
- move
- resize
- close gesture
- active/inactive appearance
- clipping of client content

### Widget priorities
1. Label
2. Button
3. Scroll bar
4. List
5. Split pane
6. Menus

### Demo target
A desktop with:
- one movable window
- title bar text in strike font
- clickable button
- scrolling list

### Exit criteria
- multiple windows overlap and redraw correctly
- focus/activation mostly correct

---

## M5 — Transcript and Workspace

### Purpose
Get the first useful development tools on screen.

## M5.1 Transcript

### Features
- append text
- scroll
- clear
- optional copy later

### Needed classes
- `TranscriptModel`
- `TranscriptView`
- `TranscriptTool`

### Demo target
System prints startup/log output to transcript window.

## M5.2 Workspace

### Features
- editable text area
- select and evaluate code
- print result
- inspect result
- transcript output integration

### Needed classes
- `TextBuffer`
- `TextSelection`
- `TextEditor`
- `WorkspaceModel`
- `WorkspaceTool`

### Demo target
Type Smalltalk into a window and execute it.

### Exit criteria
- GUI becomes self-bootstrapping for daily development
- source snippets can be executed from inside the GUI

---

## M6 — Inspector and Browser

### Purpose
Reach a real Smalltalk development environment.

## M6.1 Inspector

### Features
- object summary
- ivar list
- selected value pane
- nested inspect

### Needed classes
- `InspectorModel`
- `InspectorTool`
- `ObjectFieldListModel`

## M6.2 Browser

### Features
Minimum:
- class list
- method list
- source pane
- accept/compile source

Then add:
- protocol list
- class comment pane
- inheritance view
- senders/implementors later

### Suggested classes
- `BrowserModel`
- `BrowserTool`
- `ClassListModel`
- `MethodListModel`
- `SourceCodeController`

### Demo target
Use the Browser to edit the Browser.

### Exit criteria
- classes and methods are editable entirely from the GUI
- browser-driven development becomes normal workflow

---

## M7 — Debugger and live recovery

### Purpose
Close the loop on the classic Smalltalk experience.

### Features
- debugger opens on exception/error
- stack pane
- source pane
- temp/receiver inspector
- step / next / proceed / restart frame

### Needed classes
- `DebuggerModel`
- `DebuggerTool`
- `StackListModel`
- `ContextInspectorModel`

### VM/runtime dependencies
- good `MethodContext` exposure
- stepping support if not already present
- exception/signaling mechanism robust enough for tool integration

### Demo target
Cause an error, enter debugger, patch code, continue.

---

## M8 — Polish / expansion

### Candidates
- color display depth
- icons and bitmap resources
- package browser / change sorter
- preferences system
- file browser
- launcher/desktop menu
- UI builder or live layout editor
- multiple text styles and font sizes

---

## 3. Required Rust primitive inventory

This is the concrete host API to design next.

### Window/display
- `primitiveOpenDisplay`
- `primitiveDisplayExtent`
- `primitiveDisplayPresent`
- `primitiveDisplayResizeState`

### Events
- `primitiveNextEvent`
- `primitiveWaitForEvent:`

### Time
- `primitiveMillisClock`
- `primitiveSleep:`

### Bitmap ops
- `primitiveBitBlt`
- `primitiveFillRect`
- `primitiveInvertRect`
- `primitiveDrawLine` (optional early, likely useful)

### Font import/tooling support
Either:
- direct runtime loading of prepared font forms
or
- a one-time host-side conversion tool with produced Smalltalk-loadable font data

---

## 4. Original Smalltalk font work plan

This is now an explicit workstream.

## F0 — Identify source material
Need to locate one of:
- original ST-80 image/font assets
- archived strike font dumps
- font data from a known historical Smalltalk distribution

## F1 — Understand format
Need to determine:
- glyph bitmap storage layout
- encoding map
- widths table
- ascent/descent/baseline
- any special metadata

## F2 — Build conversion/import path
Preferred output:
- a Smalltalk-loadable font data file, or
- a compact binary/font source format our loader can ingest

## F3 — Bind into runtime
- create `StrikeFont default`
- use it for all system tools
- verify metrics with Transcript and Browser panes

## F4 — Optional later font family support
- bold/emphasis variants
- larger title fonts
- icon or symbol fonts

---

## 5. Suggested source tree expansion

This keeps GUI code organized as it grows.

### Smalltalk side
Possible future layout:

- `smalltalk/gui/geometry/`
- `smalltalk/gui/graphics/`
- `smalltalk/gui/fonts/`
- `smalltalk/gui/events/`
- `smalltalk/gui/world/`
- `smalltalk/gui/widgets/`
- `smalltalk/tools/`

Possible files:
- `smalltalk/gui/Geometry.st`
- `smalltalk/gui/Graphics.st`
- `smalltalk/gui/Fonts.st`
- `smalltalk/gui/Events.st`
- `smalltalk/gui/World.st`
- `smalltalk/gui/Widgets.st`
- `smalltalk/tools/Transcript.st`
- `smalltalk/tools/Workspace.st`
- `smalltalk/tools/Inspector.st`
- `smalltalk/tools/Browser.st`
- `smalltalk/tools/Debugger.st`

### Rust side
Possible files/modules:
- `src/display.rs`
- `src/host_events.rs`
- `src/bitmap_primitives.rs`
- `src/font_import.rs` or a standalone tool

---

## 6. Implementation order recommendation

This is the concrete recommended order of work.

1. Define Rust host display/event primitives.
2. Add minimal host window/framebuffer support.
3. Add Smalltalk `Point`, `Rectangle`, `Form`, `Display`.
4. Implement bitmap fill/copy/frame operations.
5. Acquire/import the original Smalltalk strike font.
6. Implement `StrikeFont` and text drawing.
7. Implement world loop and event queue.
8. Build one movable `SystemWindow`.
9. Build `TranscriptTool`.
10. Build text editor core.
11. Build `WorkspaceTool`.
12. Build `InspectorTool`.
13. Build `BrowserTool`.
14. Build `DebuggerTool`.

That order keeps the system useful at every stage.

---

## 7. Definition of success

We should consider the GUI effort successful when all of these are true:

1. The Smalltalk image opens a host window and draws a world.
2. The default UI uses the original Smalltalk bitmap font.
3. Windows, menus, text views, and lists are implemented in Smalltalk.
4. Transcript, Workspace, Inspector, Browser, and Debugger all run inside that GUI.
5. The Browser can browse and edit the GUI implementation itself.
6. Most daily interaction happens from inside the Smalltalk desktop, not from the external REPL.

---

## 8. Immediate next planning docs to write

To execute this roadmap cleanly, the next useful docs are:

1. `GUI-PRIMITIVES.md`
   - exact Rust <-> Smalltalk primitive interface for display/events/bitblt/time

2. `FONTS.md`
   - strike font object model
   - import format
   - original font acquisition tasks

3. `TOOLS.md`
   - exact Browser/Inspector/Workspace UI structure and data models

---

## 9. Recommended immediate next implementation step

The next actual coding step should be:

**Implement M0: host display + event primitives, and write `GUI-PRIMITIVES.md` first so the host boundary stays small and disciplined.**
