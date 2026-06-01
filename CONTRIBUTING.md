# Contributing to NovusOS

Thank you for your interest in contributing to NovusOS! This document provides guidelines for contributing to the project.

## Getting Started

1. Fork the repository
2. Clone your fork locally
3. Set up the development environment (see below)
4. Create a feature branch from `main`
5. Make your changes
6. Test thoroughly
7. Submit a pull request

## Development Environment

### Required Tools

- **Rust nightly** — automatically managed via `rust-toolchain.toml`
- **QEMU** — `qemu-system-aarch64` for testing
- **EDK2 firmware** — `edk2-aarch64-code.fd` for UEFI boot testing
- **rust-objcopy** — from `llvm-tools-preview` component

### Setup

```bash
# Clone and enter the project
git clone https://github.com/Abhinav0002/NovusOS.git
cd NovusOS

# Rust toolchain installs automatically on first build
make build           # Build the kernel
make build-novusos   # Build the UEFI desktop

# Run in QEMU
make run             # Kernel (serial)
make run-novusos     # Desktop (graphical)
```

## Project Structure

The project is organized as a Cargo workspace with three crates:

- **`kernel/`** — Bare-metal AArch64 kernel (`aarch64-unknown-none`)
- **`novusos/`** — UEFI graphical desktop (`aarch64-unknown-uefi`)
- **`userspace/init/`** — Userspace init process (`aarch64-unknown-none`)

## Code Style

- Follow standard Rust formatting (`rustfmt`)
- Use `cargo clippy` before submitting
- Prefer safe Rust where possible; document `unsafe` blocks with safety invariants
- Keep functions focused and reasonably sized
- Use meaningful names — avoid abbreviations except for well-known terms (MMU, GIC, etc.)

## Commit Messages

Write clear, descriptive commit messages:

- Use imperative mood in the subject line ("Add feature" not "Added feature")
- Keep the subject line under 72 characters
- Separate subject from body with a blank line
- Explain *what* and *why* in the body, not *how*

**Good:**
```
Implement ARP cache eviction with configurable TTL

The ARP table previously grew without bound. Add a timestamp to each
entry and evict entries older than 300 seconds on the next lookup.
```

**Avoid:**
```
fix stuff
```

## Pull Request Guidelines

- One logical change per PR
- Include a clear description of what changed and why
- Reference related issues if applicable
- Ensure the project builds without errors (`make build && make build-novusos`)
- Test on QEMU before submitting

## Areas for Contribution

### Kernel
- Write support for FAT32 filesystem
- TCP retransmission and window management
- Multi-core (SMP) boot support
- Signal handling for userspace processes
- Additional syscalls (mmap, fork, exec)

### NovusOS Desktop
- Mouse input support
- Multiple window management
- File browser application
- Text editor application
- Network status display

### Infrastructure
- CI/CD pipeline improvements
- Cross-platform build support (Linux host)
- Automated testing framework
- Performance benchmarking

## Reporting Issues

When reporting bugs, please include:

- Steps to reproduce
- Expected vs. actual behavior
- QEMU version and host OS
- Serial console output (if applicable)
- Screenshot (for graphical issues)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
