# Inspector and frame hierarchy

## Implementation plan

1. Add a collapsible right inspector: selection coordinates, dimensions and rotation; explicit Apply for one undo step; original raster dimensions and a clearly labelled PNG export size preview.
2. Add a scrollable object list with collapsible frames/groups, selection, lock and parent assignment. Keep the canvas and existing style controls available.
3. Add explicit frame membership (world coordinates, no clipping). Support nested frames, moving/scaling/rotating the selected subtree, duplication, clipboard, deletion, release and undo/redo. Prevent cycles and repair malformed file relationships.
4. Keep old documents readable. Write native format v3 only for documents containing hierarchy; old apps must reject those files instead of silently losing membership. Increase live-sharing protocol version so older peers cannot discard fields.
5. Verify core behavior, app interactions, existing suites, formatting and clippy; build release executables and inspect screenshots. Preserve the running user session and existing Korean font fix.

## Boundaries

- A frame owns explicit children. Interactive creation/paste and completed moves now infer ownership from the object's center; nested, rotated, locked and hidden frames are handled. Parent controls remain available. Subtree membership is preserved when moving an entire frame.
- Frames do not clip content. Resizing a selected subtree scales the contents; this is not Figma constraints or auto layout.
- Canvas dimensions are logical drawing units. Original bitmap pixels and PNG export pixels are labelled separately. SVG assets are vector artwork.
- Exchange formats flatten frame membership; native .inkboard files preserve it.
- No additional dependencies and no changes to the user's existing documents during development.

## Verification

### Automatic membership and frame selection follow-up (2026-11-07)

- Reproduced overlapping-frame selection failure: a nonempty title caused the entire frame background to count as a solid hit. Borders/title regions now win before container-background fallback, and clicking a selected parent's child starts an independent child drag.
- Added automatic adoption on interactive creation, paste, duplication and completed drag/keyboard movement. Ownership changes share the original undo transaction; descendant ownership is preserved. Frame-to-frame adoption requires full containment by a larger frame, preventing overlapping copies from nesting inside their sources.
- New and copied frames receive numbered names. Parent options expose full names for all frames, disabling invalid cycle/locked choices.
- Final release tests: app 380 passed (10 ignored), core library 185 passed, hierarchy integration 12 passed. Clippy passed with the same documented upstream lint allowances.

- Completed all three feature stages without new dependencies, preserving the Korean font fix.
- Workspace release suites: 1,111 tests passed and 18 ignored on the initial run; the sole failure was the CLI license inventory with uncached cross-target crates. After `cargo fetch --locked`, all CLI tests, including that inventory, passed. App suites were rerun after UI refinements.
- Core hierarchy coverage includes native round trips, malformed/cyclic parents, subtree transforms, clone/delete/release, inherited lock/visibility, and undo/redo. App coverage includes actual inspector clicks, frame dragging, title editing, clipboard, and fitting between panels.
- Formatting and whitespace checks passed. Clippy passed for app/core/session with allowances for upstream Rust-version lint incompatibilities (`unknown-lints`, `collapsible_match`, `duration_suboptimal_units`).
- Release app, CLI, and MCP binaries built. Actual desktop screenshots verified Korean text, nested object rows, frame dimensions, and export pixel preview. The existing desktop shortcut targets the rebuilt executable; the previous executable remains backed up and the running user session was not closed.
