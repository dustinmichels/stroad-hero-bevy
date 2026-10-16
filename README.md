# Stroad Hero

## Goal

I want to make an activist video game that highlights the misery of "stroads" (sprawling, high speed streets).

User plays as "Mats" a young dutch boy living in Anywhere, USA. His mother has asked him to do some errands. He is trying to do so without going splat.

It involves a frogger like element, where user has to dodge cars.

It also has a satire element, showing the sprawing urban landscape. There are parody versions of common stores like Walgroins instead of Walgreens. Taco Hell intead of Taco Bell.

I am picturing something with pixel art graphics, but not married to this idea.

I think the underlying model should perhaps be 3D, so I can simulate real traffic patterns (cars drive to their destinations, park, person gets out, gets back in, drives somewhere else.)

## Development

Requires [mise](https://mise.jdx.dev) (installs latest stable Rust) and the
[Bevy OS dependencies](https://bevy.org/learn/quick-start/getting-started/setup/#installing-os-dependencies)
(macOS: `xcode-select --install`).

```sh
mise run dev     # cargo run, Bevy dynamically linked (fast rebuilds)
mise run build   # same, build only
mise run fmt     # cargo fmt & oxfmt
cargo run --release                  # optimized native build (statically linked)
trunk build --release --cargo-profile wasm-release  # web build
```

### Web & itch.io Packaging

1. Build with relative URLs:
   ```sh
   trunk build --release --cargo-profile wasm-release --public-url ./
   ```
2. Package the `dist/` contents as a zip archive with `index.html` at the zip root:
   ```sh
   cd dist && zip -r ../stroad-hero-web.zip . && cd ..
   ```
3. Upload to [itch.io](https://itch.io) as an HTML game:
   - Check **"This file will be played in the browser"**.
   - Configure viewport dimensions to **800×800** (or matching canvas embed dimensions).
   - (Optional) Automate uploads via Butler:
     ```sh
     butler push dist <user>/stroad-hero:web
     ```

## Build configuration

Follows the [Bevy setup guide](https://bevy.org/learn/quick-start/getting-started/setup/),
plus a few additions. Each choice is also commented inline.

| Setting                                                    | Where                | Source   | Why                                                                                                                                                                   |
| ---------------------------------------------------------- | -------------------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rust = "latest"`                                          | `mise.toml`          | guide    | Bevy's MSRV is the latest stable Rust.                                                                                                                                |
| `profile.dev` `opt-level = 1`                              | `Cargo.toml`         | guide    | Debug builds of Bevy games are unplayably slow at `opt-level = 0`.                                                                                                    |
| `profile.dev.package."*"` `opt-level = 3`                  | `Cargo.toml`         | guide    | Deps compile once, so optimize them fully; our crate stays fast to rebuild.                                                                                           |
| `profile.dev.package."*"` `debug = false`                  | `Cargo.toml`         | addition | Drops dep debuginfo to cut rustc memory use on an 8 GB machine. Cost: no line info in Bevy frames when debugging.                                                     |
| `profile.release` `codegen-units = 1`, `lto = "thin"`      | `Cargo.toml`         | guide    | Slower release compiles for faster binaries.                                                                                                                          |
| `profile.wasm-release`                                     | `Cargo.toml`         | guide    | Size-optimized profile for the planned web build.                                                                                                                     |
| `log` features `max_level_debug`, `release_max_level_warn` | `Cargo.toml`         | guide    | Compiles out trace logs from dependencies; release keeps only warn/error.                                                                                             |
| `dynamic_linking` feature                                  | `Cargo.toml`         | addition | Short alias for `bevy/dynamic_linking` (the guide's biggest compile-time win). Kept opt-in rather than a default so release builds never ship needing the Bevy dylib. |
| `build.jobs = 6`                                           | `.cargo/config.toml` | addition | Caps parallel rustc on 8 GB RAM to avoid swapping. Override with `-j N` / `CARGO_BUILD_JOBS`.                                                                         |
| LLD linker (Linux, Windows)                                | `.cargo/config.toml` | guide    | Much faster linking. macOS needs nothing: default `ld-prime` is already faster.                                                                                       |
| `Cargo.lock` committed                                     | `.gitignore`         | addition | Executable crate: reproducible builds (also pins the wildcard `log` dep).                                                                                             |

Deliberately **not** used from the guide's optional fast-compile section:

- **Nightly / generic sharing**: would require a nightly toolchain; stable keeps setup simple.
- **Cranelift**: nightly-only, and the guide notes it can crash Bevy apps on macOS.
- **Mold**: Linux-only; macOS's default linker is already as fast.
