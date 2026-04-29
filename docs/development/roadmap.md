# Sutra Community — Module Roadmap

> **Status**: 21 modules (17 shipped + 4 new) | **Last Updated**: 2026-03-25

---

## Shipped Modules (17)

| Module | Domain | Actions |
|--------|--------|---------|
| sutra-nftables | Raw nftables firewall | apply_ruleset, add_rule, delete_rule, flush, list |
| sutra-sysctl | Kernel parameters | set, get, load, persist |
| sutra-aegis | Security daemon | enable, disable, status, scan, quarantine |
| sutra-daimon | Agent runtime | register, deregister, status, restart, config |
| sutra-edge | Edge fleet | provision, decommission, update, heartbeat |
| sutra-docker | Docker containers | run, stop, rm, build, pull, ps |
| sutra-podman | Podman containers | run, stop, rm, build, pull, ps |
| sutra-stiva | AGNOS containers (stiva) | run, stop, rm, build, pull, ps |
| sutra-k8s | Kubernetes | apply, delete, get, rollout, scale |
| sutra-postgres | PostgreSQL | create_db, drop_db, migrate, backup, restore, user |
| sutra-redis | Redis | config, flush, backup, info |
| sutra-nginx | Nginx | config, reload, enable_site, disable_site, certbot |
| sutra-wireguard | WireGuard VPN | setup, add_peer, remove_peer, status |
| sutra-restic | Restic backup | init, backup, restore, forget, check |
| sutra-acme | ACME/Let's Encrypt | issue, renew, revoke, status |
| sutra-cron | Cron jobs | add, remove, list, enable, disable |
| sutra-mount | Filesystem mounts | mount, unmount, fstab, list |

---

## New Modules (4) — 2026-03-25

| Module | Domain | Actions |
|--------|--------|---------|
| **sutra-nein** | Nein firewall policy | apply_policy, allow_agent, deny_agent, list_policies, container_network, status |
| **sutra-yukti** | Device management | list_devices, mount, unmount, udev_rule, format, hotplug_policy |
| **sutra-hoosh** | LLM gateway | pull_model, list_models, remove_model, set_provider, token_budget, restart, status |
| **sutra-ark** | Package management | install, update, remove, sync, verify, list, pin |

---

## Future Community Modules (demand-gated)

### AGNOS Ecosystem

| Module | Domain | Description |
|--------|--------|-------------|
| sutra-shruti | Audio workstation | Configure audio devices, PipeWire setup, MIDI config, deploy presets |
| sutra-dhvani | Audio engine | PipeWire configuration, sample rate, ALSA setup, audio routing |
| sutra-agnova | OS installer | Unattended installs, disk layout, network config, fleet provisioning |
| sutra-phylax | Threat detection | Rule deployment, scan scheduling, quarantine policy, signature updates |
| sutra-sigil | Trust management | Key distribution, trust anchor deployment, certificate rotation |
| sutra-pqc | Post-quantum crypto | PQC key generation, algorithm migration, fleet-wide rollout |
| sutra-bhava | Personality deployment | Deploy bhava personality presets to agent fleets, mood baseline config |
| sutra-murti | Model management | Pull/deploy models across fleet, VRAM-aware placement, quantization |
| sutra-goonj | Acoustics config | Room profile deployment, impulse response distribution |

### Infrastructure

| Module | Domain | Description |
|--------|--------|-------------|
| sutra-haproxy | HAProxy | Frontend/backend config, SSL termination, health checks |
| sutra-caddy | Caddy server | Reverse proxy, auto-HTTPS, Caddyfile deployment |
| sutra-mysql | MySQL/MariaDB | Database lifecycle, migrations, backup/restore, replication |
| sutra-sqlite | SQLite | Database creation, migration, backup, WAL config |
| sutra-minio | MinIO / S3 | Bucket management, access policy, replication |
| sutra-vault | HashiCorp Vault | Secret management, PKI, transit encryption |
| sutra-consul | Consul | Service discovery, KV store, health checks |
| sutra-prometheus | Prometheus | Scrape config, alert rules, recording rules, federation |
| sutra-grafana | Grafana | Dashboard provisioning, data source config, alerting |
| sutra-loki | Loki | Log ingestion config, retention policy, alerting |
| sutra-opensearch | OpenSearch | Index management, snapshots, cluster config |
| sutra-rabbitmq | RabbitMQ | Queue/exchange management, user permissions, federation |
| sutra-nats | NATS | Subject management, JetStream config, accounts |

### Platform

| Module | Domain | Description |
|--------|--------|-------------|
| sutra-systemd | systemd services | Unit management, enable/disable, timer creation, journal queries |
| sutra-grub | GRUB bootloader | Kernel parameter config, boot entry management |
| sutra-dracut | Initramfs | Rebuild initramfs, add modules, kernel cmdline |
| sutra-btrfs | Btrfs filesystem | Snapshot management, scrub, balance, subvolume operations |
| sutra-zfs | ZFS | Pool management, snapshots, send/receive, scrub |
| sutra-lvm | LVM | Volume creation, resize, snapshot, thin provisioning |
| sutra-mdadm | Software RAID | Array creation, monitoring, rebuild, spare management |
| sutra-iptables | Legacy iptables | For non-AGNOS Linux targets (nein/nftables preferred on AGNOS) |

### Networking

| Module | Domain | Description |
|--------|--------|-------------|
| sutra-tailscale | Tailscale | Join network, configure ACLs, exit node setup |
| sutra-zerotier | ZeroTier | Network join, rule config, moon setup |
| sutra-dns | DNS management | Zone files, record management, DNSSEC |
| sutra-dhcp | DHCP server | Lease management, static assignments, PXE boot |

### Cloud Providers

| Module | Domain | Description |
|--------|--------|-------------|
| sutra-aws | AWS CLI wrapper | EC2, S3, IAM, Route53, ECS operations |
| sutra-gcp | GCP CLI wrapper | Compute, Storage, IAM, Cloud Run operations |
| sutra-azure | Azure CLI wrapper | VM, Blob, AD, AKS operations |
| sutra-hetzner | Hetzner Cloud | Server, volume, firewall, network operations |
| sutra-digitalocean | DigitalOcean | Droplet, volume, firewall, database operations |

---

## Contributing a Module

1. Create `crates/sutra-{name}/src/lib.rs` implementing `SutraModule` trait
2. Implement `name()`, `actions()`, `plan()`, `apply()`
3. Add tests (minimum: module_name, module_actions, one plan test per action)
4. Add example playbook in `examples/`
5. Document in this roadmap
6. Submit PR

See existing modules (sutra-nftables, sutra-nein) for patterns.

---

*Last Updated: 2026-03-25*
