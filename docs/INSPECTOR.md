# Details and frames

Open **Details** near the upper-right corner. **Hide details** collapses the panel without changing the board.

## Geometry

Select an object to see its X/Y position, W/H dimensions and rotation. Edit the values and click **Apply geometry**. A single undo restores the entire change. For a selected frame, dimensions describe the container itself, while transforms include its children. For other multiple selections, dimensions describe the selection's bounding box and rotation is a relative rotation around its center. Canvas values are drawing units, independent of zoom.

For a selected raster image, **Original image** reports the pixel dimensions from its file header. SVG images are labelled vector artwork. The PNG preview reports the current selection's export dimensions, or the visible page content when nothing is selected, including the normal export margin and shadow allowance. Choose the same scale and selection option in the Export dialog; preview controls do not change the export dialog's settings.

## Object list

The list shows objects in reverse drawing order, with frames and groups that can be collapsed using the minus/plus buttons. Click a row to select an object, a group, or a frame and all its descendants. Shift-click adds/removes a row's selection. Click an individual child row to edit it separately. **Fit selection** centers the selection between the panels while Details is open. Lock/unlock uses the existing undoable editor operations.

## Frames

- **Frame selection** creates a named rectangular container around the selected objects. With no selection it creates an empty 640 × 480 frame at the view center.
- Select a frame and press **Enter** to edit its title.
- New frames are named `Frame 1`, `Frame 2`, and so on. Copying a frame allocates a new numbered name when its title is already used; custom title prefixes are preserved. Parent frame lists every frame by its full title, with self/descendant/locked choices disabled.
- Frame borders and title regions take precedence over overlapping container backgrounds. Selecting a child frame and dragging it does not move the previously selected parent.
- Automatic nesting of a frame requires its full rotated rectangle to fit inside a strictly larger frame. Overlapping same-sized frame copies remain independent siblings.
- **Parent frame** moves the selected roots into another frame or to **Page root**, without changing their world positions. Cycles are refused.
- Select a frame's border or its object-list row to move, resize, rotate, copy or delete its subtree. Locked descendants prevent the inspector's geometry edits and canvas dragging/deletion of the entire frame.
- **Release frame (keep contents)** removes the container and reparents its children to the enclosing frame or page. Undo restores the frame and membership.
- Drawing or pasting an object inside a frame adopts it automatically. Dragging or nudging an object uses its center to choose the innermost visible, unlocked frame; moving outside detaches it. Rotated frame interiors are respected. Moving a whole frame preserves its descendants, and moving plus reparenting is one undo step. A selected destination frame centers pasted text, images and copied objects inside it; otherwise existing paste placement is retained. **Parent frame** remains available for manual assignment. Resizing a frame around existing objects does not automatically collect them.
- Frames do not clip drawings. Resizing a selection scales its geometry; constraints, responsive layout and Figma components are outside this change. Tables retain the engine's existing inability to rotate.

## Compatibility

Old native documents still load. Documents without frame ownership retain native format v2; documents with frames/parents use v3, so old executables refuse them instead of silently discarding ownership. Keep native `.inkboard` files to preserve the hierarchy: exchange formats render the shapes but do not preserve this new ownership model.

Live-sharing protocol is v7. Host and guests need this build; the upstream hosted browser client (v6) cannot join it. No remote service is updated by this local change.
