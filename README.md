# Rusty Wrenches 🔧

[![build](https://github.com/nazar256/rusty-wrenches/actions/workflows/release.yaml/badge.svg)](https://github.com/nazar256/rusty-wrenches/actions/workflows/release.yaml)

A collection of command-line tools written in Rust to help with various system administration and file management tasks.

## Installation

### Pre-built Binaries

Download the latest release for your platform from the [releases page](https://github.com/nazar256/rusty-wrenches/releases). Available architectures:

- x86_64 Linux (musl)
- aarch64 Linux (musl)
- armv7 Linux (musleabihf)
- arm Linux (musleabi)
- powerpc64le Linux (gnu)
- i686 Linux (musl)
- s390x Linux (gnu)

### From Source

```bash
cargo install rusty-wrenches
```

Or build manually:
```bash
git clone https://github.com/nazar256/rusty-wrenches.git
cd rusty-wrenches
cargo build --release
```

## Available Tools

### fix-nested-directories

A tool that helps fix redundant nested directory structures. It's particularly useful when you have directories that are unnecessarily nested within directories of the same name.

#### Features

- Automatically fixes redundant directory nesting
- Supports dry-run mode to preview changes
- Optional name matching skip for more flexible directory restructuring
- Safe operations with logging

#### Usage

```bash
fix-nested-directories --path <DIRECTORY_PATH> [OPTIONS]
```

##### Options

- `-p, --path <PATH>`: Path to the root directory where to start searching for directories to fix
- `-s, --skip-name-match`: When specified, it will unnest folders even when they have different names than their parents
- `-d, --dry-run`: Preview changes without actually modifying the filesystem

##### Example

If you have a directory structure like this:
```
somedir/
└── somedir/
    ├── file1.txt
    └── file2.txt
```

Running the tool will restructure it to:
```
somedir/
├── file1.txt
└── file2.txt
```
