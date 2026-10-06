# vitals

[![CI](https://github.com/joanfabregat/vitals/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/joanfabregat/vitals/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/joanfabregat/vitals)](https://github.com/joanfabregat/vitals/releases/latest)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)
[![MSRV 1.85](https://img.shields.io/badge/rust-1.85%2B-orange)](https://github.com/joanfabregat/vitals/blob/main/Cargo.toml)
[![Platform: Linux](https://img.shields.io/badge/platform-linux%20x86__64%20%7C%20aarch64-lightgrey)](#platform)

A tiny Linux binary that prints CPU, memory, swap and disk usage plus the local time as one line, made for the tmux status bar.

```
cpu ▃ 34% mem ▂ 24% swap ▁ 0% disk ◕ 65% ◑ 41% · 2026-10-05 21:44
```

By default the line is tmux style markup, so each metric gets its own colour and the bars sit on a darker track. One run reads a few files under `/proc` and calls `statvfs` once per disk, so it takes about a millisecond: one process instead of the dozen a shell pipeline of `free`, `df`, `awk` and `date` starts. It has no dependencies beyond the Rust standard library and the C library.

## Install

Download the static binary for your architecture from the [latest release](https://github.com/joanfabregat/vitals/releases/latest), check it against `SHA256SUMS`, and put it on your `PATH`:

```sh
version=v0.1.0
target=x86_64-unknown-linux-musl   # or aarch64-unknown-linux-musl
base=https://github.com/joanfabregat/vitals/releases/download/$version
curl -fsSLO "$base/vitals-$target" -O "$base/SHA256SUMS"
sha256sum --check --ignore-missing SHA256SUMS
install -m 755 "vitals-$target" ~/.local/bin/vitals
```

Or build it with Cargo:

```sh
cargo install --locked --git https://github.com/joanfabregat/vitals
```

## tmux

```tmux
set -g status-interval 2
set -g status-right-length 200
set -g status-right '#(TZ=Europe/Paris ~/.local/bin/vitals) '
```

Use an absolute path: tmux runs `#()` jobs with the tmux server's environment, whose `PATH` may not include your own directories. For the same reason the server may not have your `TZ`; set it in the command as above to pin the clock's zone. Without `TZ`, the clock uses `/etc/localtime`.

## What it measures

| Metric | Source | Value |
|---|---|---|
| cpu | `/proc/stat` | Busy share of all CPUs since the previous run (see below) |
| mem | `/proc/meminfo` | `MemTotal - MemAvailable` over `MemTotal`, like `free`'s "used" |
| swap | `/proc/meminfo` | `SwapTotal - SwapFree` over `SwapTotal`; 0% without swap |
| disk | `/proc/self/mounts`, `statvfs` | One pie per filesystem, rounded like `df` |
| clock | `localtime_r` | `YYYY-MM-DD HH:MM` |

CPU usage needs two samples. Instead of sleeping, each run compares the counters with those the previous run saved in a state file (`$XDG_RUNTIME_DIR/vitals-cpu` by default), so the percentage covers the time between two refreshes. When runs come closer than half a second apart, as they do when several tmux clients refresh together, the later ones repeat the last percentage instead of measuring a few milliseconds of noise. The first run shows 0%.

Disks are the filesystems mounted from a device under `/dev/`, in mount order, once per device (bind mounts are not counted twice), skipping squashfs images and anything smaller than 10 GiB, which leaves out boot and EFI partitions.

The cpu, mem and swap bars fill in eighths (`▁` to `█`); anything under 19% still shows `▁`. Disk pies fill in quarters (`○ ◔ ◑ ◕ ●`) and switch to the alert colour at 90%.

## Options

```
      --plain             Plain text instead of tmux style markup
      --no-clock          Leave out the date and time
      --alert PERCENT     Show disks at or above PERCENT in the alert colour [default: 90]
      --min-disk-gib GIB  Skip filesystems smaller than GIB GiB [default: 10]
      --color KEY=COLOR   Set a tmux colour; repeatable. Keys: cpu, mem, swap, disk,
                          track, alert, separator, date, time
      --state PATH        CPU sample file [default: $XDG_RUNTIME_DIR/vitals-cpu]
  -h, --help              Print this help
  -V, --version           Print the version
```

Colours are tmux colour names (`red`, `colour114`) or `#rrggbb`. The defaults are cpu `colour114`, mem `colour141`, swap `colour73`, disk `colour222`, track `colour238`, alert `colour196`, separator `colour244`, date `colour39` and time `colour250`.

A metric that cannot be read is left out of the line rather than failing it.

## Platform

Linux only, on 64-bit targets. Release binaries are static musl builds for x86_64 and aarch64.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
