# SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
# SPDX-License-Identifier: AGPL-3.0-only
{
  lib,
  stdenv,
  autoreconfHook,
  gitMinimal,
  src,
  version,

  # nativeBuildInputs
  pkg-config,
  jq,
  cargo,
  rustc,
  rustPlatform,

  # buildInputs
  cargo-edit,
  cargo-deny,
  git-annex,
  just,
  taplo,
  typos,
  zsh,
}:

stdenv.mkDerivation (finalAttrs: {
  pname = "acceptarium";
  inherit version src;

  # In Nixpkgs, we don't copy the Cargo.lock file from the repo to Nixpkgs'
  # repo, but we inherit src, and specify a hash (it is a fixed output
  # derivation). `nix-update` and `nixpkgs-update` should be able to catch that
  # hash and update it as well when performing updates.
  cargoDeps = rustPlatform.importCargoLock {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = [
    autoreconfHook
    cargo
    git-annex
    gitMinimal
    jq
    pkg-config
    rustPlatform.cargoSetupHook
    rustc
    zsh
  ];

  preAutoreconf = ''
    # pretend to be a tarball release so sile --version will not say `vUNKNOWN`,
    # but don't pretend so hard we make the build system treat us like the tarball.
    echo ${finalAttrs.version} > .flake-version
    sed -i -e 's/tarball-version/flake-version/' configure.ac
  '';

  buildInputs = [
    gitMinimal
    git-annex
    zsh
  ];

  devShellInputs = [
    cargo-deny
    cargo-edit
    just
    taplo
    typos
  ];

  configureFlags = [ ];

  postPatch = ''
    patchShebangs build-aux/*.sh build-aux/git-version-gen
  '';

  strictDeps = true;

  enableParallelBuilding = true;

  # See commentary in bootstrap.sh; we're getting AMINCCLUDE stuff inlined
  # instead of included but need to avoid a file not found error on first run.
  postUnpack = ''
    touch source/aminclude.am
  '';

  outputs = [
    "out"
    "doc"
    "man"
    "dev"
  ];

  meta = {
    description = "CLI tool to facilitate digitized receipt handling in plain text accounting workflows";
    longDescription = ''
      CLI tooling to facilitate scanning receipts, extracting
      useful data, archiving the assets, and importing the results
      into Plain Text Accounting systems.
    '';
    homepage = "https://codeberg.org/plaintextaccounting/acceptarium";
    changelog = "https://codeberg.org/plaintextaccounting/acceptarium/src/branch/master/CHANGELOG.md";
    platforms = lib.platforms.unix;
    maintainers = with lib.maintainers; [
      alerque
    ];
    license = lib.licenses.agpl3Only;
  };
})
