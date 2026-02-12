# SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
# SPDX-License-Identifier: AGPL-3.0-only
{
  lib,
  libgit2,
  naersk,
  stdenv,
  cargo,
  rustc,
}:
let
  cargoToml = (builtins.fromTOML (builtins.readFile ./Cargo.toml));
in
naersk.lib."${stdenv.hostPlatform.system}".buildPackage rec {
  src = ./.;
  buildInputs = [
    cargo
    libgit2
    rustc
  ];
  cargoBuildOptions = final: final ++ [ "--features full" ];
  singleStep = true;
  copyLibs = true;
  name = cargoToml.package.name;
  version = cargoToml.package.version;
  meta = with lib; {
    description = cargoToml.package.description;
    homepage = cargoToml.package.homepage;
    license = with licenses; [ agpl3Only ];
    maintainers = with maintainers; [ ];
  };
}
