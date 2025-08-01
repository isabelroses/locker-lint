{ lib, rustPlatform }:
let
  toml = (lib.importTOML ./Cargo.toml).package;
in
rustPlatform.buildRustPackage {
  pname = "locker-rust";
  inherit (toml) version;

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.intersection (lib.fileset.fromSource (lib.sources.cleanSource ./.)) (
      lib.fileset.unions [
        ./Cargo.toml
        ./Cargo.lock
        ./src
        ./test
      ]
    );
  };

  cargoLock.lockFile = ./Cargo.lock;

  meta = {
    inherit (toml) homepage description;
    license = lib.licenses.eupl12;
    maintainers = with lib.maintainers; [ isabelroses ];
    mainProgram = "locker";
  };
}
