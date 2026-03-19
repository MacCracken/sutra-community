# Sutra Community Modules

> Community and domain-specific modules for [Sutra](https://github.com/MacCracken/sutra) infrastructure orchestration.

[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](LICENSE)

## Modules

| Module | Description | Domain |
|--------|-------------|--------|
| `sutra-nftables` | Firewall rules via nftables | System |
| `sutra-sysctl` | Kernel parameter tuning | System |
| `sutra-aegis` | Security policy enforcement | AGNOS |
| `sutra-daimon` | Agent lifecycle and fleet reporting | AGNOS |
| `sutra-edge` | Edge node operations | AGNOS |

## Why Separate?

Core sutra modules (`ark`, `argonaut`, `file`, `verify`, `shell`, `user`) work on any Linux box. Community modules are either:

- **AGNOS-specific** — aegis, daimon, edge
- **Domain-specific** — nftables, sysctl, Docker/OCI, cloud providers

This keeps sutra useful as a standalone tool beyond AGNOS, with AGNOS integration as a first-party community pack.

## Usage

Add community modules to your `Cargo.toml`:

```toml
[dependencies]
sutra-core = "2026.3"
sutra-nftables = "2026.3"
```

Then register them with your `ModuleRegistry`:

```rust
use sutra_community_nftables::NftablesModule;

registry.register(Module::Nftables(NftablesModule));
```

## Building

```bash
cargo build --workspace
cargo test --workspace
```

## License

GPL-3.0 — see [LICENSE](LICENSE).
