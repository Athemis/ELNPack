# Introduction

ELNPack is a desktop app for creating [ELN archives](https://the.elnconsortium.org) you can import into ELNs like eLabFTW. Everything runs locally—no network access required. This guide shows how to install, record an experiment, attach your data, and export the archive.

> [!TIP]
> Do all of this entirely **offline**. Ideal for use in labs with limited network access:
>
> Copy the ELN archive to a USB drive and import it into eLabFTW from another computer with proper network access.

## Prerequisites

- Prebuilt binaries are available for:
  - **Windows 10+** on **x86_64**, and **arm64**
  - **macOS** on **x86_64**, and **arm64**
  - **GNU/Linux** on **x86**, **x86_64**, and **arm64**.
- For other operating systems or architectures, you can
  [build ELNPack from source](installation.md#optional-build-from-source-developers).
- For a full list of supported OS and CPU combinations, see
  [Rust Platform Support](https://doc.rust-lang.org/beta/rustc/platform-support.html).

> [!WARNING]
> Windows XP and earlier are **not supported**. Supporting very old and current
> operating systems at the same time is difficult due to missing APIs, libraries,
> and compiler support.
>
> There is limited compiler support for Windows 7. You can try to
> [build ELNPack for Windows 7](installation.md#build-for-windows-7) yourself.

## What you can do

- Set the experiment date/time.
- Write experiment notes in the Markdown editors.
- Add keywords to your experiment.
- Describe your experiment using metadata. Import existing metadata from eLabFTW extra field JSON files.
- Attach arbitrary files.
- Export an ELN archive compatible with eLabFTW.

## License

This User Guide is licensed under the Creative Commons Attribution 4.0 International License ([CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)).
