# GUI Primitives

This document defines the current Rust ↔ Smalltalk boundary for the GUI work.

The rule is:

- Rust provides only **host/window/time/event/bitmap substrate**
- Smalltalk owns **forms, fonts, world, widgets, tools**

At the current stage, the implementation is intentionally minimal and headless-friendly.
It provides a host-display abstraction and event queue that can later be backed by a real native window.

---

## 1. Current primitive surface

## 1.1 Behavior primitives

These are installed on `Behavior` so they can be called immediately from Smalltalk.

### `Behavior hostDisplayOpenWidth: width height: height depth: depth`
Creates a host display handle and returns a `HostDisplay` instance.

Current behavior:
- creates a host display state in the VM
- returns a Smalltalk `HostDisplay` object with:
  - handle
  - width
  - height
  - depth

Notes:
- currently headless/internal
- later this should map to a native host window/framebuffer

### `Behavior hostNextEvent`
Returns the next queued host event or `nil`.

Current event encoding:
- mouse move → `#( #mouseMove x y )`
- mouse down → `#( #mouseDown x y button )`
- mouse up → `#( #mouseUp x y button )`
- key down → `#( #keyDown key )`
- key up → `#( #keyUp key )`
- resize → `#( #resize width height )`
- quit → `#( #quit )`

### `Behavior millisecondClock`
Returns monotonic milliseconds since VM startup.

### `Behavior sleepMilliseconds: milliseconds`
Sleeps for the specified number of milliseconds.
Returns `nil`.

---

## 1.2 HostDisplay primitives

### `HostDisplay>>presentForm: aForm`
Presents the contents of a `Form` to the host display.

Current behavior:
- validates that `aForm` is a `Form`
- reads:
  - width
  - height
  - depth
  - bits
- stores the last presented framebuffer bytes in the VM host display state
- increments present count

Supported form depths currently:
- 1-bit packed bitmap
- 8-bit byte-per-pixel buffer
- other depths are not yet validated rigorously and should be considered provisional

### `HostDisplay>>savePNG: aPathString`
Saves the last presented framebuffer to a PNG file.

Current behavior:
- reads the saved host display snapshot
- writes an 8-bit grayscale PNG
- for 1-bit displays:
  - `0` bits are saved as white
  - `1` bits are saved as black

This is the key headless inspection path for GUI development.

---

## 2. Current Smalltalk GUI kernel classes

Defined in:
- `smalltalk/gui/Kernel.st`

Currently provided:
- `Point`
- `Rectangle`
- `Form`
- `HostDisplay`

These are intentionally minimal bootstrap classes.

---

## 3. Current Form representation

A `Form` currently has ivars:
- `width`
- `height`
- `depth`
- `bits`

`bits` is currently a `ByteArray`.

### Depth 1 form layout
For `depth = 1`:
- bits are packed into bytes
- required byte size = `ceil(width * height / 8)`
- currently computed in Smalltalk as:
  - `((width * height) + 7) bitShift: -3`

### Depth 8 form layout
For `depth = 8`:
- one byte per pixel
- required byte size = `width * height`

---

## 4. What is implemented right now

Implemented:
- host display object creation
- host event queue API
- monotonic clock API
- sleep API
- `Form` bootstrap class with bit storage allocation
- `HostDisplay>>presentForm:` plumbing
- `HostDisplay>>savePNG:` for headless inspection
- testable headless host display snapshots in Rust

Not yet implemented:
- real native host window
- actual OS event pumping
- BitBlt copy/fill primitives
- line drawing
- cursor management
- damage repair/compositor
- font loading/rendering

---

## 5. Why the current host layer is headless

This is deliberate for now.

Reasons:
1. It keeps the primitive boundary stable while the GUI object model is developed.
2. It makes tests deterministic and CI-friendly.
3. It lets us build `Form`, `World`, and event logic before binding to a specific host-window crate.

The intended evolution is:

1. keep the Smalltalk-level API the same
2. replace the internal headless host display backend with a real native window backend
3. preserve all higher-level Smalltalk code unchanged

---

## 6. Immediate next primitive work

Next additions should be:

1. `primitiveBitBlt`
   - rectangle copy between forms
2. `primitiveFillRect`
   - fill region in form with pattern/color
3. host window backend
   - actual window creation and framebuffer presentation
4. host event backend
   - real mouse/keyboard/resize events

Once those land, the Smalltalk side can move to:
- world loop
- window drawing
- text rendering with strike fonts

---

## 7. Example current usage

```smalltalk
Behavior hostDisplayOpenWidth: 640 height: 480 depth: 1
```

```smalltalk
Display := Behavior hostDisplayOpenWidth: 64 height: 32 depth: 1.
Buffer := Form new initializeWidth: 64 height: 32 depth: 1.
Display presentForm: Buffer.
Display savePNG: 'display.png'.
```

```smalltalk
Now := Behavior millisecondClock.
Behavior sleepMilliseconds: 10.
Later := Behavior millisecondClock.
```

```smalltalk
Event := Behavior hostNextEvent.
```
