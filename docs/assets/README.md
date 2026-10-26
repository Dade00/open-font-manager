# Screenshot provenance

These PNGs were captured from the real Windows Tauri development app, not a browser mock or a recreated layout. Capture used the existing `target/x86_64-pc-windows-msvc/debug/open-font-manager-app.exe`, the current Vite frontend at port 1420, and the WebView2 Chrome DevTools Protocol endpoint. The Rust binary was reused, not rebuilt for this documentation task.

All three images are 2160 × 1380 pixels (a 1440 × 920 CSS-pixel viewport at the machine's scale). No application source changes or synthetic font data were used.

- `local-library.png`: existing local library, Georgia Italic selected; the inspector reported a real loaded preview.
- `google-fonts.png`: existing cached Google Fonts catalog, Montserrat Regular selected, with a loaded preview. The stale-cache warning is intentionally retained.
- `font-pairing.png`: an unsaved pairing draft with Georgia Regular for the heading and Arial Regular for the body. The draft was not saved; existing saved user content remains visible below it.

The capture machine's fonts, collection names, existing composition and local data path are visible. These are not bundled project assets or sample datasets. Font files themselves are not distributed with these screenshots.

At startup the app displayed “Impossibile accedere ai dati locali dell'applicazione.” The notification was dismissed through its close button after the library loaded; this does not establish that startup maintenance succeeded. This documentation task did not diagnose that error or validate Windows installation/uninstallation, ZIP imports, clipboard operations or moodboard persistence end to end.

To refresh screenshots, run the native app against the current frontend, navigate with its real controls, wait for previews to finish, and capture the WebView2 viewport. Keep cache warnings and unsaved-state labels honest. Avoid changing a user's persistent data merely to stage screenshots.
