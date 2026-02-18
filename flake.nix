# SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
# SPDX-License-Identifier: AGPL-3.0-only
{
  description = "CLI tool to facilitate digitized receipt handling in plain text accounting workflows";

  # To make user overrides of the nixpkgs flake not take effect
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.gitignore = {
    url = "github:hercules-ci/gitignore.nix";
    inputs.nixpkgs.follows = "nixpkgs";
  };

  # https://wiki.nixos.org/wiki/Flakes#Using_flakes_with_stable_Nix
  inputs.flake-compat = {
    url = "github:edolstra/flake-compat";
    flake = false;
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      flake-compat,
      gitignore,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
        inherit (gitignore.lib) gitignoreSource;
        # https://discourse.nixos.org/t/passing-git-commit-hash-and-tag-to-build-with-flakes/11355/2
        version_rev = if (self ? rev) then (builtins.substring 0 7 self.rev) else "dirty";
        acceptarium = pkgs.callPackage ./build-aux/pkg.nix {
          version = "${(pkgs.lib.importTOML ./Cargo.toml).package.version}-${version_rev}-flake";
          src = pkgs.lib.cleanSourceWith {
            # Ignore many files that gitignoreSource doesn't ignore, see:
            # https://github.com/hercules-ci/gitignore.nix/issues/9#issuecomment-635458762
            filter =
              path: type:
              !(builtins.any (r: (builtins.match r (builtins.baseNameOf path)) != null) [
                # Nix files
                "flake.nix"
                "flake.lock"
                "default.nix"
                "shell.nix"
                # git commit and editing format files
                ".editorconfig"
                # Git files
                ".git"
              ]);
            src = gitignoreSource ./.;
          };
        };
      in
      rec {
        devShells = {
          default = pkgs.mkShell {
            inherit (acceptarium)
              buildInputs
              nativeCheckInputs
              ;
            nativeBuildInputs = acceptarium.nativeBuildInputs ++ acceptarium.devShellInputs;
            configureFlags = acceptarium.configureFlags ++ [
              "--enable-developer-mode"
            ];
          };
        };
        packages = {
          acceptarium = acceptarium;
          acceptarium-clang = acceptarium.override {
            # Use the same clang version as Nixpkgs' rust clang stdenv
            stdenv = pkgs.rustc.llvmPackages.stdenv;
          };
        };
        defaultPackage = packages.acceptarium;
        apps = rec {
          default = acceptarium;
          acceptarium = {
            type = "app";
            program = "${self.defaultPackage.${system}}/bin/acceptarium";
          };
        };
        defaultApp = apps.acceptarium;
      }
    );
}
