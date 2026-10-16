# Changelog

All notable changes to Discard are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/).

## [0.3.0] - 2026-10-16

> **Breaking: all members must update.** v0.2.0 nodes can't talk to v0.3.0 nodes.
> Every message, voice signal and workspace change is now signed, and unsigned
> frames from older versions are dropped, so chat, calls and workspace sync only
> work between members who are all on 0.3.0.

**Upgrading:** install 0.3.0 manually from the release assets (as with
earlier versions). From 0.3.0 on, Discard tells you when a new release is out
and can install it and restart from inside the app.

### Added

- Rename and delete workspaces and channels as any member, from a settings menu
  next to the workspace name and from hover buttons on each channel. Changes
  reach everyone, including members who were offline at the time.
- Leave a workspace on this device only, without deleting it for others.
- Shared soundboard: custom sounds you upload are available to every member of
  your workspaces, and anyone can rename or delete them. Uploads are limited to
  1 MB of audio and the library holds up to 100 sounds.
- Text chat inside voice channels, available whether or not you are in the call.
- Center stage for calls: while viewing the voice channel you are in, video
  takes the main area and chat moves to the side. Click a tile (or press
  Enter/Space) to spotlight it; Esc, the close button or another click returns
  to the grid. Screen shares are shown uncropped.
- Join and leave chimes when someone enters or leaves your voice channel
  (silent while deafened).
- Screen sharers see who is watching their share and hear a short cue when
  someone starts watching.
- In-app updates: a banner announces new releases, and on installed builds
  (Discard.app on macOS, the `discard` binary on Linux) one click downloads,
  verifies and installs the update, then restarts Discard.

### Changed

- Workspace and channel edits converge on every member with a
  last-writer-wins rule; deletions always win, so a deleted channel or
  workspace never comes back from a stale or concurrent edit.
- After a restart, a node reconnects to its workspaces through every member it
  knows about, not only the workspace owner, and closes its network connection
  cleanly on quit so peers notice the restart right away.
- The node reports its version in the log and at `GET /api/version`.

### Fixed

- Channels created after an invite was issued now show up for every member.
- Pressing Enter when creating a channel no longer creates it twice.

### Security

- All workspace traffic (messages, voice signaling and presence, workspace and
  channel edits, shared sounds) is signed with the sender's identity key and
  verified on receipt; a member can no longer impersonate another member.
- Sound edits carry the editor's own signature, so a relaying member cannot
  forge or alter someone else's change, and an edit cannot swap the audio of an
  existing sound.
- The built-in media relay (TURN) only accepts connections from peers seen in
  voice signaling, limits allocations, clients and packet sizes, expires idle
  allocations, and compares credentials in constant time.
- Updates are downloaded over HTTPS from GitHub releases only and checked
  against the published size and SHA-256 digest before installing. The update
  endpoints require the session token.

## [0.2.0] - 2026-10-16

### Added

- Calls work across any NAT: WebRTC media is relayed over the existing iroh
  link through a built-in loopback TURN server, so no public TURN server is
  needed. `DISCARD_RELAY_ONLY` forces the worst-case relayed path for testing.
- Release builds attach macOS and Linux binaries to the GitHub release.

### Fixed

- The app keeps its connection to the node alive, and you stay in your voice
  channel across brief disconnects.
- Reloading the page or leaving and rejoining a call reconnects video and audio
  instead of leaving a stale connection.
- Healthy calls are no longer torn down when presence briefly lags.
- Messages no longer show as encrypted text until a refresh (workspace keys are
  sent when the app connects).

## [0.0.1] - 2026-10-15

First public build: local-first, peer-to-peer text chat, voice and video,
including the fix for gossip stalling on frames over 4 KiB (the limit is now
64 KiB, so large call offers and long messages get through).

[0.3.0]: https://github.com/JonRoosevelt/discard/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/JonRoosevelt/discard/compare/0.0.1...v0.2.0
[0.0.1]: https://github.com/JonRoosevelt/discard/releases/tag/0.0.1
