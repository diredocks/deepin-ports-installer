{
  description = "graphical image installer for deepin (ports)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };

        electronLibraries = with pkgs; [
          # GTK / GLib
          gtk3
          glib

          # Chromium
          nss
          nspr
          atk
          at-spi2-atk
          cups
          dbus
          expat
          libdrm
          mesa
          libxkbcommon
          libglvnd
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            nodejs
            pnpm
            pkg-config

            # electron-forge/native modules
            python3
            gnumake
            gcc

            # linter and formatter
            oxfmt
            oxlint
          ];

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath electronLibraries;
        };
      }
    );
}
