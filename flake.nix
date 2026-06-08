{
  description = "Device management dependencies";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-26.05";
    nixpkgs-unstable.url = "github:NixOS/nixpkgs/nixos-unstable";
    command-utils.url = "git+https://codeberg.org/expede/nix-command-utils";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      fenix,
      flake-utils,
      nixpkgs,
      nixpkgs-unstable,
      command-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        pkgsUnstable = import nixpkgs-unstable { inherit system; };
        toolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain-nightly.toml;
          # pkgs.lib.fakeSha256
          sha256 = "sha256-v+i2vvBAKg14CNWODfuTQ5ikMo43vEOznqXy6vAb8WA=";
        };

        cmd = command-utils.cmd.${system};
        asModule = command-utils.asModule.${system};
        
        info = {
          "devices:connected:info" = cmd "List connected devices" "probe-rs list";
        };

        rp-project = "./platform/2350";
        rp = {
          "device:rp235x:bbqueue-alloc-psram:build" =
            cmd "Build RP235X Blink Async" "(cd ${rp-project}/bbqueue-alloc-psram; cargo run --bin bbqueue-alloc-psram --release)";
        };

        command_menu = command-utils.commands.${system} [ (asModule (rp // info))];
      in
      {
        formatter = pkgs.nixfmt-rfc-style;

        devShells = {
          default = pkgs.mkShell {
            name = "Embassy examples";

            shellHook = "menu";

            buildInputs = [
              # nix
              pkgsUnstable.nixd

              # Microcontroller
              pkgsUnstable.probe-rs-tools

              # Rust
              pkgsUnstable.rust-analyzer
              toolchain

              # Commands
              command_menu
            ];
          };
        };
      }
    );
}