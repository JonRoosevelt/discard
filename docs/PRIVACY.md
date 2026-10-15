# Discard — Privacy and threat model

First-class requirement: **there is no central server storing messages**. All content
lives on the participants' devices and is replicated **peer-to-peer**. This document defines what we
promise, what we do not promise, and why.

## Principles

1. **Local-first.** The user's device is the source of truth. There is no central database.
2. **No content server.** No operator sees messages, files, audio or video.
3. **End-to-end encryption by default.** It is not optional or "enterprise-only".
4. **No third parties.** No telemetry, analytics, CDN, font or call to an external service.
   Fonts are the system's; every asset is local.
5. **Metadata minimization.** We collect the minimum and do not log content.
6. **Verifiable.** Only dependencies that can be audited; no opaque binaries.

## Data retention

**We do not store messages.** They are ephemeral: they live at most **24 hours**, only in the local buffer of
each node (memory + local cache), and are discarded automatically. There is no durable history, there is no
history synchronization between devices, and nothing is sent for storage. Attachments and soundboard audios
follow the same 24h window and are collected (GC) afterwards. Only the **workspace configuration**
(name, channels) persists locally, because it is not conversation content.

## Threat model

| Adversary | What they want | How we defend | What remains |
|---|---|---|---|
| Operator of a central server | Read messages, usage profile | There is no content server | — |
| MITM on the network | Read/write traffic | QUIC + TLS 1.3, keys verified by cryptographic identity | — |
| Peer of a workspace | Read channels without access | Access to content is a **capability** (document key); without the key, the data is opaque | A legitimate member sees what was granted to them |
| Relay operator (fallback) | Correlate, read content | The relay only forwards **already encrypted packets**; it never sees content | Sees EndpointIDs, IPs and timing (see Metadata) |
| Network observer / ISP | Metadata | Encrypted QUIC; configurable DHT/discovery | Volume, timing, relay IPs |
| Malicious peer | Expose IP, flood | Direct connections are the user's choice; rate-limit and local blocking | IP is visible to those who connect directly (see below) |

## Privacy architecture

### Transport — `iroh`
Each node is an `iroh::Endpoint` identified by a **cryptographic key** (ed25519), not by IP.
Connecting is "dialing by key". Every connection is **QUIC + TLS 1.3**, authenticated and encrypted end to end.
Public relays exist only as a **fallback** for NAT traversal and forward **already encrypted** packets;
no data is stored in the cloud.

### Identity
The user's identity is a **keypair generated on the device**. The private key **never leaves the
device** (IndexedDB in the browser client; local file with `0600` permission in the node). There is no
email, phone or server account. The `display_name` is local and unauthenticated.
Backup = **manual export** of a recovery file (the key is not derived from the password).

### Content confidentiality
- **Messages and state:** live in an **iroh Document** (multiwriter KV via CRDT). The document's
  identity is the public key of a pair that **controls writing**. Sharing the `DocTicket` is the act
  of granting access. Without the ticket, the document is unreachable.
- **Files:** go to the content-addressed **blobs store** (BLAKE3). The hash only means something to
  whoever already has access to the document that references it.
- **Media:** in a WebRTC mesh, audio/video is encrypted end to end by **DTLS-SRTP** between the peers.
  The local node is not a media intermediary.
- **Defense in depth (implemented):** the body of each message is encrypted in the browser with
  **AES-256-GCM** (WebCrypto) before leaving the UI, so that the local node and the database store only
  ciphertext. The key is derived from the workspace `secret` (which already comes in the `DocTicket`) via
  **HKDF-SHA256** with `info="discard-content-v1"`. Envelope: `"E1:" + base64(nonce(12) || ciphertext)`.
  Peers that have the workspace secret decrypt it; those that do not see opaque content. If `secret`
  is not available, the text is sent unencrypted (compatibility fallback).
  **Scope:** protects the content **in transit and at rest** (gossip, blobs, disk) and **against peers
  without the key** — not against the local node itself: being yours, it knows the `secret` and is not a central
  server. The **attachment bytes** are not yet encrypted in the client; they go as a blob, whose access depends
  on the workspace ticket.

### Encryption at rest
- Local node: document and blob store on disk; the identity key protected by permissions and,
  optionally, a local passphrase.
- Browser: private key and sensitive material in IndexedDB; nothing in `localStorage` beyond the session
  token of the local node (which is ephemeral and local).

## Metadata — what still leaks (honesty)

1. **IP between direct peers.** Direct connections (a product choice, lower latency) reveal the IP to
   whoever you connect to. Those who want to hide their IP can require a relay; it is planned as an option per
   workspace. **It is not the default.**
2. **Discovery.** By default `iroh` uses n0's DNS/relay to locate peers. For maximum privacy:
   **self-host a relay** and/or **lookup via Mainline DHT** (`BEP 44`), both configurable.
3. **Timing and volume.** A fallback relay sees when and how much you exchange, even though it does not see what.
4. **Presence.** Being online and in a channel is visible to the members of that workspace.

## What we do NOT promise

- Network anonymity (the IP is exposed to direct peers and relays).
- Resistance to an adversary that controls **all** your peers and your device.
- That members of a channel will not leak content (they have legitimate access by definition).
- Cryptographic deniability (document entries are **signed** by the author).

## Decisions that sustain privacy

- **No central server** — there is no target to seize; there is no "we see your messages".
- **Capability-based access** — access is a key, not a permission record on a server.
- **Zero third parties** — no Google Fonts/Analytics/STUN.
- **Keys only on the device** — compromising the server does not compromise the user.

## Verification checklist

- [ ] No network request to third-party domains (audit `web/` and deps).
- [ ] Private key never serialized to the node/network.
- [ ] No content log (messages, file bodies, SDP/ICE) at `info` level.
- [ ] Relays and lookup self-hostable via configuration.
- [ ] Per-workspace option to "force relay / hide IP".
- [ ] Key backup export/import.
