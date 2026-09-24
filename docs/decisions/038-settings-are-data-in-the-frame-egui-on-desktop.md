# 38. Settings are data in the frame; egui on desktop, painted by us (decided)

Logged 2026-09-21.

The renderer held its own `Effects`, set by a method only the headless shot
called. A UI needs them on the seam, so they moved: `scene::Effects` is plain
data, a UI sends `set_effects`, the client clamps and answers
`effects_changed`, and every `Frame` carries what is in force. The renderer
keeps no setting. Weather had been a function of the clock alone; with the wind
a knob that would jump the sky, so the renderer now advances it frame by frame
in f64, and starts from the clock alone on a first frame or a set clock, which
keeps a headless shot a function of its clock.
Desktop UI is egui (closes the question in OPEN), in its own crate behind the
seam, as CLAUDE.md asks of integrations. `egui-wgpu` pins wgpu 29 and we ride
30, as vybe found: egui and egui-winit come from crates and the painter is ours,
some 300 lines, since egui's contract is textured triangles and scissors.
Rejected: holding wgpu back for a UI crate. On the web the choice is kept in
`localStorage`, not the account: a phone and a desktop want different answers.
