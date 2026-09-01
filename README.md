<div align="center">

# 🕳️ Void

**Peer-to-peer messaging over Tor. No servers. No accounts. No traces.**

*Messagerie pair-à-pair au-dessus de Tor — votre identité est une adresse oignon.*

Discord-like · End-to-end encrypted · Windows 10/11 x64 · Rust + Tauri

English · Français (Settings → Language)

</div>

---

## ✨ Features

- **Identity = `.onion` address**: every peer is a v3 onion service derived from an ed25519 key. No registry, no account — a 24-word BIP39 phrase is all it takes to recover your identity on any machine.
- **Direct P2P connections**: peers dial each other through Tor (signed ed25519 handshake, sessions with ping/RTT, automatic reconnection).
- **Encrypted direct messages**: X25519 (ECDH) → HKDF-SHA256 → ChaCha20-Poly1305, with delivery receipts and local history (SQLite).
- **Voice notes**: record up to 60s (Opus 16 kb/s), sent as encrypted messages — offline relay, history and receipts included.
- **Sender-key groups**: 256-bit conversation key shared over pairwise encrypted channels, automatic key rotation on member removal, P2P fan-out.
- **Offline delivery (store-and-forward)**: online peers hold encrypted envelopes (7-day TTL, anti-abuse quotas) and deliver them when the recipient returns. Local queue + resynchronization on reconnect.
- **Zero network leakage**: strict CSP, 100% local assets, `SocksPort OnionTrafficOnly` (the embedded proxy rejects everything except `.onion`), offline WebView2 installer. Only outbound traffic: tor.exe to the Tor network.
- **"The void" interface**: black & white theme, animated starfield, Space Grotesk + Inter, notifications, autostart, signed updates (minisign) via GitHub Releases — strictly manual checking.

## 📦 Installation

Download `Void_x.y.z_x64-setup.exe` from the [Releases](../../releases) page (~260 MB: Tor and WebView2 bundled, per-user install, nothing downloaded at install time).

First launch: Void starts its embedded Tor relay, generates your identity and shows your invite link (`void://invite?...` + QR code). Exchange it with a peer, add them via "+" — the conversation begins.

> ⚠️ **Back up your recovery phrase** (My identity → Recovery phrase). Without it, your identity is unrecoverable. The seed is encrypted via DPAPI (bound to your Windows session).

## 🔐 Security model

| Element | Mechanism |
|---|---|
| Identity | ed25519 → `.onion` v3 (SHA3 checksum), DPAPI-protected seed, BIP39 |
| Handshake | Signed HELLO/WELCOME, domain-separated, public key extracted from the announced address (spoofing impossible) |
| DMs | X25519 ECDH → HKDF conversation key (salt = both sorted addresses) → ChaCha20-Poly1305 with bound AAD |
| Groups | Pairwise-shared key, rotation on removal, per-message-id deduplication |
| Relays | E2E-unreadable envelopes, 1000 max / 200 per sender / 256 KB / 7-day TTL |
| Network | Strict CSP, local fonts, OnionTrafficOnly, ClientOnly 1, no telemetry |

**Known limitations**: no forward secrecy (static conversation keys), messages stored in plaintext in the local SQLite (DB encryption planned), metadata (sender/recipient/size) visible to relays, Tor latency (~300 ms).

## 🛠️ Development

Prerequisites: Rust stable MSVC, Node.js 20+, npm.

```powershell
git clone https://github.com/lucas-rivil/void.git
cd void
scripts/fetch-tor.ps1        # Tor expert bundle (resources)
scripts/make-icon.ps1        # icon
cd apps/desktop
npm install
npm run tauri dev            # dev window with hot-reload
```

Architecture (Rust multi-crate workspace):

```
crates/void-crypto   identities, .onion derivation, ECDH/AEAD, BIP39
crates/void-proto    wire frames, handshake, DM/group envelopes, relaying
crates/void-tor      embedded tor process + ControlPort
crates/void-store    SQLite (messages, relay queue)
crates/void-core     Engine: sessions, groups, relays, sync
apps/desktop         Tauri v2 + React + Tailwind ("the void" UI)
```

Headless tests (real Tor network):

```powershell
cargo run -p void-core --example smoke     # tor + onion service
cargo run -p void-core --example torwatch  # kill tor.exe → auto restart
cargo run -p void-core --example p2p       # peer-to-peer connection
cargo run -p void-core --example dm        # encrypted messages + receipts
cargo run -p void-core --example groups    # groups: invite, removal
cargo run -p void-core --example relay     # store-and-forward (3 engines)
cargo run -p void-core --example restore   # identity restoration
```

## 🚀 Publishing a release (maintainers)

1. Bump `version` in `apps/desktop/src-tauri/tauri.conf.json`
2. `git tag vX.Y.Z && git push origin vX.Y.Z` — GitHub Actions builds and publishes the signed release
3. Users: Settings → Update → "Check"

Locally signed build: `scripts/release.ps1` (minisign key in `~/.tauri/void-updater.key`).

## 🤝 Contributing

Issues and PRs welcome. Dev environment: `npm run tauri dev`, tests: `cargo test --workspace` + the headless examples above.

## ⚖️ License

MIT — see [LICENSE](LICENSE).
