## Web Shell Manual Verification

1. Start the daemon with terminal routes enabled:
   `AMUXD_TERMINAL_RENDERER_V1=1 cargo run --manifest-path amuxd/Cargo.toml`
2. Open `http://127.0.0.1:8080/app` in a desktop-width browser.
3. Register a non-git workspace and verify:
   the workspace appears in the rail as `none`
   the local-session form targets that workspace
   the managed-worktree panel explains that worktrees are unavailable
4. Create a local session for the non-git workspace and confirm it is auto-selected, the route changes to `/app/sessions/{session_id}`, and the session label shows `workspace-name · local`.
5. Register a git workspace and verify:
   the workspace appears as `git`
   the managed-worktree form loads both local and remote tracking source refs
   the selected workspace panel shows the repo root path
6. Create a managed worktree from a local branch, then create another from a remote tracking branch, and confirm both appear in the tracked worktree list with their branch name, source ref, and generated `.amux-worktrees/...` path.
7. Start a session from one of the managed worktrees and confirm the selected session label shows `workspace-name · worktree:branch-name`.
8. Type text into the terminal input box, send it, and verify the terminal canvas refreshes immediately.
9. Terminate the selected session and confirm the UI normalizes back to `/app` with the unavailable banner.
10. Reload a valid `/app/sessions/{session_id}` route and confirm the same session is restored.
11. Disable terminal routes by restarting without `AMUXD_TERMINAL_RENDERER_V1`, reopen `/app`, select a session, and confirm the terminal-unavailable state appears while workspace selection, local session creation, worktree creation, and session termination still work.
12. Repeat the flow at a phone-sized viewport width and confirm:
   the rail is reached through the `Workspaces` drawer button
   workspace registration, local session creation, and managed worktree launch controls remain usable
   the `Ctrl`, `Esc`, `Tab`, arrow, and `Enter` buttons remain reachable
