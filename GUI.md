# Smalltalk GUI Plan

This document lays out a plan for a **fully Smalltalk-implemented GUI** on top of the VM.
The goal is a living Smalltalk desktop with classic tools:

- world / desktop
- windows
- menus
- text editor
- workspace
- transcript
- inspector
- debugger
- browser / class browser / method browser
- eventually package and change tools

The implementation should preserve classic Smalltalk strengths:

- everything is live objects
- the GUI is inspectable and modifiable from inside itself
- tools are built from the same widget/composition framework they expose
- no hard-coded application UI in Rust
- the Rust side provides only the minimum host integration primitives

---

## 1. Design goals

### 1.1 Primary goals

1. **GUI in Smalltalk, not Rust**
   Rust should only provide framebuffer, input, time, and a few acceleration primitives.

2. **Fully bitmapped**
   The display is a bitmap. Rendering is performed by Smalltalk object code plus a few low-level bitmap primitives.

3. **Bitmap fonts**
   Text should use strike/bitmap fonts first. Vector fonts can be deferred.

4. **Dynamic and extensible**
   Windows, widgets, menus, browsers, inspectors, and editors should all be open to runtime modification.

5. **Classic Smalltalk feel**
   The system should support a browser-centric environment, integrated tools, live update, and object inspection/debugging.

### 1.2 Non-goals for the first GUI milestone

- hardware acceleration
- anti-aliased vector text
- modern CSS/layout systems
- complex international text shaping
- GPU compositing
- accessibility stack beyond basic keyboard/mouse handling
- networked UI remoting

These can come later.

---

## 2. Architecture overview

The GUI stack should be split into clear layers.

### Layer 0: VM host surface
Minimal Rust-hosted functionality:

- open a native window
- expose a framebuffer to Smalltalk
- pump mouse / keyboard / resize / quit events
- provide monotonic clock / timer tick
- optionally clipboard and file dialogs later

This layer should know nothing about widgets, windows, browsers, or text editors.

### Layer 1: BitBlt / Form graphics kernel
Smalltalk-visible graphics objects and primitives:

- `Display`
- `Form`
- `Cursor`
- `Rectangle`
- `Point`
- `Color` or initial monochrome/palette abstraction
- `BitBlt` or equivalent blitter object

Responsibilities:

- copy bits between forms
- fill rectangles
- draw lines / frames
- render glyph bitmaps
- support clipping
- support dirty rectangle accumulation

### Layer 2: Font subsystem
Objects for:

- `StrikeFont`
- `GlyphForm`
- `TextStyle`
- `TextRun` / attributed text later

Responsibilities:

- glyph lookup
- metrics: ascent, descent, width, line height
- bitmap glyph rendering
- font fallback policy
- text measurement

### Layer 3: Input and event dispatch
Objects for:

- `InputSensor`
- `EventQueue`
- `Hand` / pointer state
- keyboard focus manager

Responsibilities:

- poll native events from the VM
- convert to Smalltalk event objects
- dispatch events into the world
- manage mouse capture, drag tracking, keyboard focus

### Layer 4: World / compositor / scheduler
Objects for:

- `World`
- `Window`
- `View` / `Morph` / `Pane` base class
- `DamageRecorder`
- `Stepper` / timer-driven updates

Responsibilities:

- maintain the desktop/world object graph
- z-order of windows
- invalidation and redraw
- dragging and resizing
- periodic stepping / animation hooks

### Layer 5: Widget framework
Objects for:

- labels
- buttons
- scroll bars
- lists
- text areas
- menus
- split panes
- tab panes later

Responsibilities:

- composition
- hit testing
- layout
- keyboard navigation
- model/view/controller or morph-like behavior

### Layer 6: Tools
Classic Smalltalk tools built in the framework:

- Transcript
- Workspace
- Inspector
- Browser / Class Browser
- Debugger
- Change List / Package Browser later

---

## 3. Core model: MVC vs Morphic-like

We should choose an approach that preserves Smalltalk style while staying simple.

### Option A: classic ST-80 MVC
Pros:

- historically aligned with classic Smalltalk
- naturally supports browser-style tools and pluggable views
- simpler mental model for the first tools

Cons:

- can become rigid for arbitrary interactive widgets
- more boilerplate for direct manipulation

### Option B: Morphic-like retained object scene
Pros:

- very dynamic
- easier composition and live editing
- easier drag/drop and arbitrary embedded tools

Cons:

- more rendering/event complexity
- can drift from the simpler browser/tool-first milestone

### Recommended path
Use a **hybrid**:

- retained visual objects for rendering and event dispatch
- MVC/pluggable-model conventions for tools

Concretely:

- define a retained base visual object (`View` or `Morph`)
- tools such as browser/inspector/workspace should still separate model from view where it helps
- avoid overcommitting to a huge Morphic clone too early

---

## 4. Graphics plan

## 4.1 Display representation

Start with a **1-bit monochrome display**, then extend to palette or 32-bit color later.

Why monochrome first:

- historically Smalltalk-like
- smallest surface area
- simplest BitBlt implementation
- best fit for bitmap fonts
- easiest to debug image save/load and drawing correctness

Then add:

- 2-bit / 4-bit indexed color, or
- directly 32-bit ARGB display forms

Recommended intermediate plan:

1. first GUI: monochrome
2. second pass: indexed palette
3. later: 32-bit color

## 4.2 Forms

Need objects representing rectangular pixel arrays:

- width
- height
- depth
- words/bytes backing store
- optional offset/origin metadata

Forms required:

- the global display framebuffer
- offscreen forms for double-buffering and glyphs
- cached icons and cursors
- window backing stores if needed

## 4.3 BitBlt primitives

Minimum primitive set:

- copy rectangle source->dest
- fill rectangle with solid pattern
- draw frame rectangle
- invert rectangle
- glyph blit with mask or transparent background
- clipping rectangle support

Optional but useful early:

- source alpha / transparent color key for color mode
- line draw primitive
- scroll region primitive

The key rule: the primitive operates on bitmaps, not on UI concepts.

---

## 5. Font plan

## 5.1 Font object model

Implement a classic strike-font system.

Suggested classes:

- `AbstractFont`
- `StrikeFont`
- `GlyphMap`
- `TextStyle`
- `FontFamily` later

A `StrikeFont` should contain:

- glyph bitmap storage
- per-glyph widths
- ascent/descent
- baseline
- default char / missing glyph
- encoding map

## 5.2 About the “original Smalltalk font”

Technically: **yes, we can support it**.

But there are two distinct issues:

1. **technical acquisition**
   - fonts can be extracted from historical Smalltalk images / strike font tables
   - or imported from archived ST-80 material if format details are known

2. **redistribution rights**
   - exact Xerox PARC Smalltalk-80 font assets may have unclear redistribution status
   - shipping them in the repo may be legally murky unless their license is verified

### Working assumption for this project

For now, we assume it is acceptable to use the **original Smalltalk strike font** if we can extract it from historical Smalltalk material.

So the immediate plan becomes:

- design the text system around `StrikeFont`
- obtain the historical Smalltalk font data from an image/archive or equivalent strike-font source
- import that font into our native `StrikeFont` representation
- use it as the initial system font for Transcript, Workspace, Browser, and other tools

And also support:

- importing additional historical strike fonts from external files
- a conversion tool that extracts strike font data into our `StrikeFont` format
- later swapping in recreated/open fonts if needed

### Practical answer

- **Yes, we should actively target the original Smalltalk font now**.
- The GUI should be designed so the original strike font becomes the first-class system font, not just an optional extra.

## 5.3 Font file/import formats

Support at least one import path:

- custom `.strike` dump format
- BDF import for bitmap font authoring
- converter from historical image/font data later

BDF import is especially useful because:

- many bitmap fonts already exist
- easy to inspect and version
- good for generating an initial system font

---

## 6. Window system plan

The window system should be implemented inside Smalltalk.

Suggested core classes:

- `World`
- `Desktop`
- `Window`
- `SystemWindow`
- `Panel`
- `ScrollPane`
- `Menu`
- `MenuItem`

Responsibilities:

- window bounds
- title bar drawing
- close/collapse/menu controls
- drag and resize
- activation/focus
- clipping to client area
- damage invalidation

Initial model:

- single world
- multiple overlapping windows
- active window with keyboard focus
- mouse-driven move/resize

---

## 7. Widget framework plan

We need a compact but expressive widget base.

Suggested class tree:

- `View`
  - `CompositeView`
  - `TextView`
  - `ListView`
  - `ButtonView`
  - `ScrollBarView`
  - `MenuView`
  - `IconView`
  - `SplitView`

Core protocols:

- `drawOn:`
- `bounds`
- `bounds:`
- `subviews`
- `addSubview:`
- `removeSubview:`
- `layoutSubviews`
- `invalidate`
- `handleEvent:`
- `wantsKeyboardFocus`
- `mouseDown:` / `mouseMove:` / `mouseUp:`
- `keyDown:`

Layout:

Start simple:

- fixed bounds
- split panes
- list/text intrinsic sizing
- explicit layout managers later

Then add:

- row layout
- column layout
- proportional split layout
- minimum/preferred sizes

---

## 8. Text system plan

A real Smalltalk GUI lives or dies on text editing.

## 8.1 Text model

Suggested objects:

- `Text`
- `TextBuffer`
- `TextSelection`
- `TextEditor`
- `TextView`
- `ParagraphFormatter` later

First milestone can use plain strings plus selection ranges.

Later add:

- attributed text
- emphasis/style runs
- syntax highlighting
- undo/redo

## 8.2 Editing features required early

- insertion/deletion
- selection
- caret movement
- mouse selection
- scroll
- clipboard later
- line-wise editing
- basic shortcuts

## 8.3 Text rendering

Required:

- measure glyphs
- line break by width
- caret painting
- selection inversion/highlight
- tab support later

---

## 9. Tool plan

## 9.1 Transcript
First GUI tool.

Why first:

- simple scrolling text output
- useful for debugging the GUI itself
- validates text rendering and scrolling

Needed features:

- append text
- auto-scroll
- clear
- basic menu

## 9.2 Workspace
Second tool.

Needed features:

- editable text area
- “do it” / “print it” / “inspect it” actions
- selection-based evaluation
- transcript integration

This is the bridge from textual bootstrap to live GUI development.

## 9.3 Inspector
Third tool.

Needed features:

- object summary pane
- instance variable list
- selected field value pane
- ability to inspect deeper
- evaluate expressions in object context later

## 9.4 Browser / Class Browser
Core flagship tool.

Suggested panes:

1. package/category pane later
2. class pane
3. protocol pane
4. method list pane
5. source text pane

Early version can start with:

- class list
- method list
- source pane
- compile / accept button

Then add:

- protocol/categories
- class comment pane
- hierarchy view
- references/senders/implementors later

## 9.5 Debugger
Needed once exceptions and breakpoints are usable.

Panes:

- stack list
- source pane
- temp/ivar inspector pane

Actions:

- step
- step over
- proceed
- restart frame
- inspect receiver/context

The debugger is essential for a true Smalltalk environment.

---

## 10. Extensibility model

This is where the system should feel like Smalltalk.

Principles:

1. **Tools are ordinary objects**
2. **Menus are data and behavior objects**
3. **Widgets are subclassable live**
4. **Windows can host arbitrary tools**
5. **The browser can browse the GUI system itself**

Needed protocols:

- registration of tools in a launcher/menu
- pluggable inspectors for domain objects
- command pattern or menu action objects
- window/tool factories

Potential classes:

- `ToolRegistry`
- `Command`
- `Action`
- `ToolBuilder`
- `BrowserModel`
- `InspectorModel`

---

## 11. Rust-side support required

The Rust side should stay minimal but must expose enough power.

## 11.1 Required host primitives

- create/show native host window
- framebuffer present/update
- get framebuffer extent
- poll next input event
- sleep / wait for event with timeout
- monotonic millisecond clock

## 11.2 Required bitmap primitives

- BitBlt copy/fill/invert
- maybe line drawing
- optional scroll region primitive

## 11.3 Optional host conveniences

Later only:

- clipboard get/set
- open/save file dialog
- drag/drop file paths
- system cursor switching

---

## 12. Suggested Smalltalk class set

A concrete starting class inventory:

### Geometry / graphics
- `Point`
- `Rectangle`
- `Color`
- `Form`
- `Display`
- `BitBlt`
- `Cursor`
- `DamageRecorder`

### Fonts / text
- `StrikeFont`
- `TextStyle`
- `TextBuffer`
- `TextSelection`
- `TextView`
- `TextEditor`

### Events / world
- `InputSensor`
- `Event`
- `MouseEvent`
- `KeyboardEvent`
- `World`
- `Hand`
- `Window`
- `SystemWindow`

### Widgets
- `View`
- `CompositeView`
- `LabelView`
- `ButtonView`
- `ListView`
- `ScrollBarView`
- `MenuView`
- `SplitView`

### Tools
- `TranscriptTool`
- `WorkspaceTool`
- `InspectorTool`
- `BrowserTool`
- `DebuggerTool`
- `LauncherTool`

### Models / support
- `ToolRegistry`
- `Command`
- `SelectionModel`
- `ListModel`
- `BrowserModel`
- `InspectorModel`

---

## 13. Delivery phases

## Phase 0: prerequisites

Before GUI work begins in earnest:

- stable source loading with the in-image compiler
- enough collections/strings/streams for tool implementation
- file loading and saving support for source and images

## Phase 1: display kernel

Deliver:

- host window + framebuffer primitive support
- `Display`, `Form`, `Rectangle`, `Point`
- simple BitBlt operations
- screen clear and rectangle drawing

Demo:

- Smalltalk draws test patterns into the host window

## Phase 2: fonts and text drawing

Deliver:

- `StrikeFont`
- glyph blit
- text measurement
- line drawing of strings
- initial system font

Demo:

- render text labels and transcript lines

## Phase 3: events and world

Deliver:

- mouse/keyboard event objects
- world loop in Smalltalk
- invalidation/damage redraw
- cursor tracking

Demo:

- clickable test widgets in the world

## Phase 4: windows and widgets

Deliver:

- `Window`
- `View` hierarchy
- buttons, lists, scrollbars, menus
- text view/editor base

Demo:

- movable windows with controls

## Phase 5: transcript and workspace

Deliver:

- transcript window
- workspace with do-it / print-it / inspect-it

Demo:

- live code evaluation from GUI

## Phase 6: inspector and browser

Deliver:

- object inspector
- class browser / method browser
- source accept/compile from GUI

Demo:

- browse and modify the GUI system from the browser itself

## Phase 7: debugger and live development loop

Deliver:

- debugger UI
- exception integration
- source navigation from stack frames

Demo:

- crash into debugger, patch code, continue

## Phase 8: polish and expansion

Potential additions:

- color UI
- icons
- package/change tools
- preferences
- file browser
- image snapshot UI
- visual object editor / live UI builder

---

## 14. Risks and mitigations

### Risk: too much in Rust
Mitigation:
- enforce a narrow primitive boundary
- keep UI policy and widgets in Smalltalk only

### Risk: text editor becomes the bottleneck
Mitigation:
- prioritize text system early
- keep first editor simple but solid

### Risk: font asset licensing uncertainty
Mitigation:
- ship an open compatible bitmap font first
- support external import of historical fonts

### Risk: self-hosted compiler not strong enough for tooling complexity
Mitigation:
- build required collections/text support first
- develop tools incrementally: Transcript -> Workspace -> Inspector -> Browser

### Risk: event/render architecture gets overcomplicated
Mitigation:
- start monochrome
- start single world / single host window
- start retained object tree + simple invalidation

---

## 15. Recommended immediate next steps

1. Add a new design doc for host display primitives and event primitives.
2. Define the minimal Rust primitive API for:
   - display extent
   - framebuffer present
   - next event
   - time
   - BitBlt
3. Design Smalltalk classes for:
   - `Point`
   - `Rectangle`
   - `Form`
   - `Display`
   - `StrikeFont`
   - `World`
   - `View`
4. Implement a monochrome display test in Smalltalk.
5. Implement text rendering with a bundled open bitmap font.
6. Build Transcript first.
7. Then Workspace.
8. Then Inspector and Browser.

---

## 16. Bottom line

The right path is:

- **Rust provides a tiny bitmap-and-event substrate**
- **Smalltalk implements the graphics model, world, widgets, tools, and browsers**
- **bitmap fonts first, with support for importing a historical Smalltalk font later**
- **Transcript/Workspace/Inspector/Browser form the first true Smalltalk desktop**

That gives us a real Smalltalk environment: not just a VM that runs code, but a live system that can browse, edit, inspect, and grow itself.
