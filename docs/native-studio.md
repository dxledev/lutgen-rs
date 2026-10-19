# Native Studio display and theme

Studio uses the window system selected by winit, preferring native Wayland when
available. This renders at the monitor's pixel density, including fractional
scaling on Hyprland. To select a backend explicitly, launch with
`LUTGEN_STUDIO_UNIX_BACKEND=wayland lutgen-studio` or
`LUTGEN_STUDIO_UNIX_BACKEND=x11 lutgen-studio`.

The generated `theme.toml` exposes `colors.slider_track` for the unfilled slider
track. It is derived from the desktop surface color and adjusted against both
the window and panel backgrounds. `accessibility.minimum_track_contrast`
defaults to `2.0`, with a minimum of `1.5`. Tracks are opaque so their contrast
does not depend on the compositor background.

Existing theme files without `slider_track` derive it from their own surface
color. Custom theme edits remain in effect until a desktop theme source changes.
Run `lutgen-studio --generate-theme` to regenerate the complete theme file.
