# Install & Launch

## Easiest: download the release binary

1. Go to the project’s Releases page on GitHub.
2. Download the asset for your platform (Windows, macOS, or Linux) and CPU (x86_64, i686, or aarch64/arm64).
3. Extract the archive and run the executable.

### Windows (10+)

> [!IMPORTANT]
> Binaries are not code-signed; if SmartScreen appears, choose “More info” → “Run anyway”.

1. Download the Windows `.zip` (x86_64 or i686):
   - `elnpack-x86_64-pc-windows-msvc.zip` (64-bit)
   - `elnpack-i686-pc-windows-msvc.zip` (32-bit)
2. Extract it (right-click → Extract All…).
3. Double-click `elnpack.exe`.
   - If the Universal CRT is missing, install the latest VC++ Redistributable (x64 or x86).

### macOS (Intel & Apple Silicon)

> [!IMPORTANT]
> Binaries are not code-signed; if Gatekeeper blocks execution, using “Open” once will allow it to run.

1. Download the macOS `.tar.gz` matching your Mac:
   - Apple Silicon → `elnpack-aarch64-apple-darwin.tar.gz`
   - Intel → `elnpack-x86_64-apple-darwin.tar.gz`
2. Double-click to extract (or run `tar -xzf elnpack-*.tar.gz`).
3. In Finder, right-click the `elnpack` app/binary → Open → confirm.

### GNU/Linux

1. Download the Linux `.tar.gz` for your CPU (x86_64, i686, or aarch64):
   - `elnpack-x86_64-unknown-linux-gnu.tar.gz`
   - `elnpack-i686-unknown-linux-gnu.tar.gz`
   - `elnpack-aarch64-unknown-linux-gnu.tar.gz`
2. Extract and run:

```bash
tar -xzf elnpack-<version>-<arch>-unknown-linux-gnu.tar.gz
cd elnpack-<version>
chmod +x elnpack
./elnpack
```

3. Needs glibc ≥ 2.31 (e.g., Ubuntu 20.04+). On minimal systems ensure `libc6`, `libgcc-s1`, and `libm` exist.

## Build from source

### Prerequisites

- [Installed](https://www.rust-lang.org/tools/install) and working stable Rust compiler toolchain (≥ 1.85.0; Rust 2024)
- You intend to build for a [supported build target](https://doc.rust-lang.org/beta/rustc/platform-support.html).

### Build & Run

Building and running ELNPack from source is straightforward. Ensure you have the prerequisites in place and follow these steps:

```bash
# Clone the repository
git clone https://github.com/Athemis/ELNPack.git && cd ELNPack
# Compile and run a debug build
cargo run
# Compile and run a release build
cargo run --release
```

#### Windows 7

> [!IMPORTANT]
> Windows 7 is a so-called _[Tier 3](https://doc.rust-lang.org/beta/rustc/target-tier-policy.html#tier-3-target-policy)_ build target, which means it is **not officially supported** by Rust. Our testing has shown that it is possible to build and run ELNPack on Windows 7, however there are some known issues with the Windows 7 build target ([see below](installation.md#known-issues)).
>
> Furthermore, building requires a nightly toolchain.

The `*-win7-windows-msvc` targets can only be built on Windows and need the [MSVC BuildTools](https://visualstudio.microsoft.com/en/downloads). If you have problems compiling or running the produced binary, try installing an older version of the BuildTools such as `MSVC v140 - VS 2015 C++ build tools (v14.00)` from the installer. Make sure to install the `Windows SDK` as well.

Unfortunately, the `*-win7-windows-gnu` targets which in theory would allow cross compilation and are not dependent on the MSVC BuildTools, produce binaries that are not compatible with Windows 7 due to the lack of certain system libraries.

```bash
# Install Rust Nightly Toolchain
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly

# The MSVC targets can only be built on Windows and need the MSVC BuildTools:
# https://visualstudio.microsoft.com/en/downloads

# Build binaries; use your desired target
# cargo +nightly build -Z build-std --target x86_64-win7-windows-msvc
cargo +nightly build -Z build-std --target i686-win7-windows-msvc
```

##### Known Issues

> [!NOTE]
> Limited testing was performed in **virtualized** Windows 7 systems due to a lack of physical hardware running this OS.
>
> Some or all of the following issues may be solely caused by the state of virtualized graphics hardware driver support and may or may not be absent on real hardware.

- The **mouse pointer** is off by a significant amount. While the application remains functional, you may have a hard time hitting the intended interface elements.
- There were **severe graphical distortions**, when the window is not maximized. Work around this by maximizing the window before interacting with it.
- The application will not launch without functional **OpenGL support** due to requirements of the underlying user interface library.
