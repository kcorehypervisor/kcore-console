# Local appliance console. The same binary serves a kcore hypervisor node or a
# kcore Vault backup host; `product` selects the labels and the units it checks.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.kcore-console;
in
{
  options.services.kcore-console = {
    enable = lib.mkEnableOption "kcore appliance console";

    package = lib.mkOption {
      type = lib.types.package;
      description = "Package providing bin/kcore-console.";
    };

    product = lib.mkOption {
      type = lib.types.enum [
        "hypervisor"
        "vault"
      ];
      default = "hypervisor";
      description = "Which appliance this screen describes.";
    };

    tty = lib.mkOption {
      type = lib.types.str;
      default = "/dev/tty1";
      description = "TTY the console attaches to.";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];

    # The console replaces the local login prompt on the appliance display.
    services.getty.autologinUser = lib.mkForce null;
    services.getty.helpLine = lib.mkForce "";
    systemd.services."autovt@".enable = lib.mkForce false;
    systemd.services."getty@tty1".enable = lib.mkForce false;
    systemd.services."getty@tty2".enable = lib.mkForce false;
    systemd.services."getty@tty3".enable = lib.mkForce false;
    systemd.services."getty@tty4".enable = lib.mkForce false;
    systemd.services."getty@tty5".enable = lib.mkForce false;
    systemd.services."getty@tty6".enable = lib.mkForce false;
    systemd.services."serial-getty@ttyS0".enable = lib.mkForce false;
    systemd.services."serial-getty@hvc0".enable = lib.mkForce false;

    # The binary reads this file when `--product` is not passed.
    # `kcore-vault` selects the backup appliance; anything else is a hypervisor node.
    environment.etc."kcore-type".text = if cfg.product == "vault" then "kcore-vault\n" else "kcore\n";

    systemd.services.kcore-console = {
      description = "kcore appliance console (${cfg.product})";
      after = [
        "local-fs.target"
        "network-online.target"
      ];
      wants = [ "network-online.target" ];
      wantedBy = [ "multi-user.target" ];
      before = [ "getty.target" ];
      unitConfig.Conflicts = [
        "getty@tty1.service"
        "autovt@tty1.service"
      ];
      serviceConfig = {
        Type = "simple";
        Restart = "always";
        RestartSec = "2s";
        ExecStart = "${cfg.package}/bin/kcore-console --tty ${cfg.tty}";
        User = "root";
        Group = "root";
        Environment = "PATH=${
          lib.makeBinPath (
            with pkgs;
            [
              coreutils
              iproute2
              util-linux
              systemd
            ]
          )
        }:/run/wrappers/bin";
        UMask = "0077";
        NoNewPrivileges = true;
        ProtectSystem = true;
        ProtectHome = true;
        PrivateTmp = true;
        StandardInput = "tty";
        StandardOutput = "tty";
        StandardError = "journal";
        TTYPath = cfg.tty;
        TTYReset = true;
        TTYVHangup = true;
        TTYVTDisallocate = true;
      };
    };
  };
}
