# Build & Install

> Status: dev/build commands are **Implemented**. The GitHub Actions release
> matrix is described as the intended packaging setup.

Nexus Notes is a Tauri v2 app: a Rust backend plus a Vite/React frontend.
JavaScript deps are managed with **pnpm**.

## Prerequisites

### All platforms
- **Node.js** 18+ and **pnpm** (`npm i -g pnpm`).
- **Rust** (stable, edition 2021; `rust-version = 1.77`) via [rustup](https://rustup.rs).
- Tauri v2 CLI is provided as a dev dependency (`@tauri-apps/cli`), invoked via
  `pnpm tauri …` — no global install required.

> The browser preview (`pnpm dev`) needs only Node + pnpm. Rust and the native
> deps below are only required to build/run the **desktop** app.

### Linux (e.g. Ubuntu 22.04)
Install the WebKitGTK and related system libraries Tauri links against:

```bash
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  build-essential curl wget file \
  libxdo-dev libssl-dev \
  libayatana-appindicator3-dev librsvg2-dev
```

### macOS
- **Xcode Command Line Tools**: `xcode-select --install`.
- WebView is provided by the system **WKWebView** (no extra runtime).

### Windows
- **Microsoft Visual Studio C++ Build Tools** (MSVC toolchain).
- **WebView2** runtime (preinstalled on current Windows; the bundler can also
  embed it).

## Install

```bash
pnpm install
```

## Develop

### Browser preview (no Rust needed)
```bash
pnpm dev          # vite — http://localhost:5173
```
In a plain browser, `src/lib/ipc.ts` serves every command from an **in-memory
mock vault**, so the entire UI renders and is screenshot-verifiable without the
backend.

### Full desktop app (Rust + WebView)
```bash
pnpm tauri dev    # = tauri dev: launches the native window with HMR
```
This runs `beforeDevCommand` (`pnpm dev`) against `devUrl`
`http://localhost:5173` (per `src-tauri/tauri.conf.json`).

Useful: set `NEXUS_VAULT=/path/to/vault` to launch straight into a vault (the
`startup_vault` command reads it). A sample vault ships at
`examples/demo-vault/`.

### Type-check / frontend build
```bash
pnpm build        # tsc --noEmit && vite build  → dist/
```

## Package (production binaries)

```bash
pnpm tauri build  # = tauri build: runs `pnpm build`, then bundles the app
```

`tauri.conf.json` sets `bundle.targets: "all"`, so on each OS Tauri produces that
platform's native installers:

| Platform | Artifacts |
|----------|-----------|
| Linux | `.AppImage`, `.deb` |
| Windows | `.exe` (NSIS), `.msi` |
| macOS | `.dmg` (and `.app`) |

App identity: product **Nexus Notes**, identifier **com.nexusnotes.app**,
version **0.1.0**.

## Release matrix (GitHub Actions)

Cross-platform packaging is intended to run as a CI matrix that builds each
installer on its native runner (Tauri cannot cross-compile these GUI bundles):

```yaml
jobs:
  release:
    strategy:
      matrix:
        include:
          - os: ubuntu-22.04   # → .AppImage, .deb
          - os: windows-latest # → .exe, .msi
          - os: macos-latest   # → .dmg
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20, cache: pnpm }
      - uses: dtolnay/rust-toolchain@stable
      # Linux only: install libwebkit2gtk-4.1-dev + the deps listed above
      - run: pnpm install
      - run: pnpm tauri build
      # upload installers as release assets (e.g. tauri-apps/tauri-action)
```

> Note: there is no `.github/workflows/` in the repo yet; the above is the
> documented intended setup.

## Honest platform note

**Each desktop installer must be built on its own OS.** Tauri's bundlers depend
on platform-native tooling (WKWebView/codesign on macOS, MSVC/WebView2 on
Windows, WebKitGTK on Linux), so:

- **macOS** `.dmg` builds only on **macOS**.
- **Windows** `.exe`/`.msi` build only on **Windows**.
- On **Linux** you can build the Linux `.AppImage`/`.deb` (and run `pnpm tauri
  dev`), but you **cannot** produce macOS or Windows binaries.

This is why release packaging uses the multi-OS GitHub Actions matrix above
rather than a single build host.
