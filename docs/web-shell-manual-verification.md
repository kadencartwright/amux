## Web Shell Manual Verification

1. Start the daemon with terminal routes enabled:
   `AMUXD_TERMINAL_RENDERER_V1=1 cargo run --manifest-path amuxd/Cargo.toml`
2. Open `http://127.0.0.1:8080/app` in a desktop-width browser.
3. Create a session, confirm it is auto-selected, and verify the route changes to `/app/sessions/{session_id}`.
4. Type text into the terminal input box, send it, and verify the terminal canvas refreshes immediately.
5. Select a different session from the rail and confirm the terminal surface switches without waiting for the next poll interval.
6. Terminate the selected session and confirm the UI normalizes back to `/app` with the unavailable banner.
7. Reload a valid `/app/sessions/{session_id}` route and confirm the same session is restored.
8. Disable terminal routes by restarting without `AMUXD_TERMINAL_RENDERER_V1`, reopen `/app`, select a session, and confirm the terminal-unavailable state appears while create/select/terminate still work.
9. Repeat the flow at a phone-sized viewport width and confirm:
   the session rail is reached through the `Sessions` drawer button
   the `Ctrl`, `Esc`, `Tab`, arrow, and `Enter` buttons remain reachable
   create/select/terminate controls remain touch-accessible
