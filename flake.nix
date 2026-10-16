{
  description = "kcore appliance console for the hypervisor and for Vault";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          kcore-console = pkgs.rustPlatform.buildRustPackage {
            pname = "kcore-console";
            version = "0.5.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            KCORE_GIT_REV = self.shortRev or self.dirtyShortRev or "dev";
          };
        in
        {
          default = kcore-console;
          inherit kcore-console;
        }
      );

      nixosModules.default = import ./modules/kcore-console.nix;
      nixosModules.kcore-console = import ./modules/kcore-console.nix;
    };
}
