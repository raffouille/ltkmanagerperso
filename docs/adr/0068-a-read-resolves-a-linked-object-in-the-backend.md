# ADR-0068: A read resolves a linked object in the backend

- **Status:** Accepted (2026-10-22)
- **Date:** 2026-10-22
- **Crates:** `ltk-manager-game`, where `lookup::ObjectLookup` holds the open bin and the bins it links,
  and `src-tauri`, where `read_lookup` builds one per command
- **Related:** "The Conditions section" in `docs/ux/SKIN_EDITOR.md`, whose effects are declared in
  linked bins for most skins.

## Context and problem statement

A bin lists the bins it links, and an object that it references can be declared in any of them. A
skin keeps most of its particle systems, and often its resolver, its animation graph and some
materials, in shared bins such as `Ahri_Multi_Skins_*.bin`. Ahri's `skin0.bin` declares no
particle system of its own.

Each read handled the links its own way. `read_vfx_system` failed for a system in a linked bin.
`read_skin` read the resolver only from the open bin. The graph, the material and the child system
reads each walked the links with their own cap, so one call parsed a linked bin up to three times.
The models carried a `source` field naming the linked bin an object was found in, and the frontend
opened a second document handle on that bin to read the object again. Which bin a tab opened came
from the object index, whose ranking differs from the walk.

## Decision

**A consumer names an object by its hash against the open document.** `vfxQueries.system`,
`skinQueries.graph` and `readMaterialPrograms` take the open document and a hash, and the backend
read finds the bin that declares it. No value model carries the bin an object was found in.

**One `ObjectLookup` per command.** `ObjectLookup` answers the open bin first, then reads the linked bins breadth
first as the read asks for objects, each bin at most once, at most 32 per command. The VFX, skin,
graph and material program reads all resolve through it, and a read that names a bin by
convention, such as the skin bin of a system's path, adds it to the same lookup.

**A requested object fails, a reference skips.** The object a command is asked for fails with
`BIN_NODE_NOT_FOUND` if no bin of the lookup declares it. A child system, a material or an effect
system that it references and that no bin declares is skipped.

**A tab or an edit asks `locate_object`.** Opening an object in a tab, or editing a material that
another bin declares, needs that bin. `locate_object` answers it with the same lookup the read used,
so the tab opens the bin that the preview drew.

**A row read of an object that another bin declares answers typed data.** The row reads are per
document. Where the open document does not declare the resolver or the animation graph, its row
read answers `BIN_NODE_NOT_FOUND`, and the view draws the typed read, read-only: the effect tables
take the skin read's `effectSystems`, and a clip takes its `GraphClip` fields.

## Consequences

- **Positive:** a skin whose systems, resolver or graph sit in linked bins previews the same as one
  that declares them itself, and a linked bin is parsed once per command.
- **Positive:** the frontend holds no handle on a linked bin for a read, so no second document
  opens and closes with the views.
- **Negative:** the effects of a skin whose resolver another bin declares, and the clips of a graph
  that another bin declares, take no edits in the skin view. They are edited in a tab of that bin.
- **Negative:** `MaterialPreview` no longer says which bin declares a material, so the material
  pane asks `locate_object` before it draws its rows.
