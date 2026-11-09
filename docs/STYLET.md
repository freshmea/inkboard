# Validating the pen on real hardware

*Version française : [fr/STYLET.md](fr/STYLET.md)*

Pen pressure and the pen eraser are **implemented and automatically tested, but have never been tried with a real pen** (no digitizer was
available during development). The diagnostic mode lets you check it in two minutes.

## Procedure (Windows)

```bash
inkboard --pen-test
```

A "Pen diagnostics" panel appears on the right. On a pen-enabled device (Surface, Wacom tablet, pen touchscreen…):

1. **Draw** with the Pen tool, varying the pressure: the stroke should get thicker as you press. In the panel, "Pen detected by native driver",
   "Variable pressure" and "Pressure received by the application (egui)" should turn **yes**.
2. **Flip the pen** (eraser end) and pass over a drawing: it should be erased, whatever the active tool. "Eraser end detected" → yes.
3. Press the **barrel button** ("Barrel button detected") and tilt the pen ("Tilt detected"): informational.
4. Click **Copy the report** and paste it into a message: it contains the verdicts and the measurements (number of pressure levels, etc.).

## Linux (X11 / XWayland)

Use **Inkboard (Wacom)** from the application menu, or run `inkboard-wacom --pen-test` after the source install.
Without installing, run `env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET ./target/release/inkboard --pen-test`.
The current window library does not handle native Wayland tablet events and drops X11 pressure valuators. Inkboard therefore
queries the kernel's live Wacom pen state, normalizes the hardware's advertised pressure range and passes it to the drawing engine.
Direct state queries continue updating while a compositor/window-library grab owns the drawing gesture.
Run `sudo ./scripts/setup-wacom-linux.sh` once, then restart the application. The udev rule grants the active local user access only
to Wacom pen nodes, excluding tablet pads, keyboards and other pointing devices. The application runs as the normal user and never
grabs or changes the input device. Mouse strokes remain constant-width when the pen is lifted.

The diagnostic panel shows **Native pressure** and the pressure received by the application. Both should vary while drawing;
the stored stroke keeps one pressure value for each point. Highlighter strokes deliberately keep a constant width.
On Ubuntu, `libinput-tools` and `libwacom-bin` can diagnose the system driver separately. `xsetwacom` configures an Xorg session;
it cannot configure tablets managed by a Wayland compositor.

## Reading the results

| Observation | Meaning |
|---|---|
| "Pressure received by Windows" yes but "by the application (egui)" NO | Windows sees the pressure but egui does not pass it on: variable width will not work; the fix is to read the pressure directly from `inkboard-pen` (already available in `PenSample`). |
| "Variable pressure" NO with few levels | The driver returns a binary pressure (0/1) or the mouse is emulating the pen: check the driver / Windows Ink. |
| "Eraser end" NO although the eraser end is used | The driver does not set `PEN_FLAG_INVERTED` / `PEN_FLAG_ERASER`; some pens have no hardware eraser. |
| Last pointer "mouse" while using the pen | Windows Ink is disabled for the application or the driver emulates the mouse. |

## What is verified without hardware

- The interpretation of the pen flags (`inkboard-pen`) and the construction of the verdicts (`inkboard-app/src/pen_test.rs`).
- The installation of the window *subclass* on a dummy window that does not alter the messages.
- The application's behavior when the eraser is detected (the gesture erases whatever the active tool) and the rendering of width according to pressure.
