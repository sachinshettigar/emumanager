# Screenshots

The repo README embeds these:

| File | Screen |
| --- | --- |
| `hero.png` | Dashboard beside a running emulator |
| `dashboard.png` | Dashboard — an emulator with live state |
| `create.png` | Create wizard — the device step |
| `dependencies.png` | Dependencies & host-readiness |
| `profiles.png` | Profiles — import a `.emuprofile` |

## Refreshing them

1. Run a build (`just dev`), size the window to a normal desktop shape.
2. Capture each screen: **macOS** `Cmd+Shift+4`, then `Space`, then click the window.
3. Drop the raw PNGs here as `_raw-<name>.png` (`_raw-hero.png`, `_raw-create.png`, …).
4. Tweak each crop box in [`polish.py`](polish.py), then run it
   (`pip install --user Pillow` once):

   ```sh
   python3 docs/screenshots/polish.py
   ```

It writes the final `<name>.png` — cropped to the window, rounded corners, a soft
shadow on a transparent margin so it reads on light and dark GitHub themes. The
`_raw-*.png` inputs are ignored by git; only the finished PNGs are committed.
