# kcore console

Read-only local display for a kcore machine. One binary, two products:

- `hypervisor` — node agent, controller, and dashboard (default)
- `vault` — Vault coordinator, dashboard, and mover

```bash
kcore-console --product hypervisor --dev
kcore-console --product vault --tty /dev/tty1
```

`KCORE_CONSOLE_PRODUCT` sets the same switch. `KCORE_API_PORT` overrides the
probed port (9091 on a hypervisor node, 9191 on a Vault coordinator).

## NixOS

```nix
services.kcore-console = {
  enable = true;
  product = "vault"; # or "hypervisor"
  package = inputs.kcore-console.packages.${system}.default;
};
```

The module is `kcore-console.nixosModules.default`. kcore and kcore-vault import
it and set `product`.

## Build

```bash
nix build
cargo test
```
