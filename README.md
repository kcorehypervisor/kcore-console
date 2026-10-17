# kcore console

Read-only local display for a kcore machine. One binary, two products:

- `hypervisor` — node agent, controller, and dashboard (default)
- `vault` — backup appliance: coordinator, dashboard, and mover

```bash
kcore-console --dev
kcore-console --product vault --tty /dev/tty1
```

The product is chosen in this order:

1. `--product` or `KCORE_CONSOLE_PRODUCT`
2. `/etc/kcore-type` — `kcore-vault` selects the backup appliance
3. hypervisor, when neither is set

`KCORE_API_PORT` overrides the probed port (9091 on a hypervisor node, 9191 on
a Vault coordinator).

## Pages

| Key | Page | Contents |
| --- | --- | --- |
| 1 | Overview | Node, health, and access cards, plus the system log. A vault host shows the backup repository instead of the health card. |
| 2 | Network | Interfaces. IPv4 is never truncated. The detail card lists every IPv6 address. |
| 3 | Storage | Disks, with model, mounts, and role in the detail card. |
| 4 | GPU | PCI display and accelerator devices, IOMMU group, driver, and whether the device is bound to `vfio-pci` for passthrough. |
| 5 | Security | TPM, Secure Boot, firmware identity, IOMMU, and kernel hardening, each marked pass, warn, or fail. |
| 6 | Diagnostics | The local API probe and the product's systemd units. |
| 7 | Help | Keys and the notes for this appliance. |

`Tab` and the left/right arrows change page. `↑` `↓` (and `j` `k`) move the
selection or scroll the log. `PgUp` `PgDn`, `Home` `End`, and `g` `G` jump.
`r` refreshes immediately; inventory also refreshes every 5 seconds. `?` or
`h` opens Help. `Esc` returns to Overview. `q` and `Ctrl+C` quit only in
`--dev` mode.

## Logo

The header wordmark is `assets/kcore-logo.png`. `build.rs` turns it into
half-block cell art at build time, so the console stays inside the Linux
framebuffer font. Terminals shorter than 24 rows get the text wordmark.

## NixOS

```nix
services.kcore-console = {
  enable = true;
  product = "vault"; # or "hypervisor"
  package = inputs.kcore-console.packages.${system}.default;
};
```

The module writes `/etc/kcore-type` and starts the console without `--product`.
It is `kcore-console.nixosModules.default`. kcore and kcore-vault import it and
set `product`.

## Build

```bash
nix build
cargo test
```
