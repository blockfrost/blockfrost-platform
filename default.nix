{ pkgs ? import
    # move to stable after rust 1.99 is stable
    (builtins.fetchTarball {
      url = "https://github.com/NixOS/nixpkgs/archive/a58bc8ad779655e790115244571758e8de055e3d.tar.gz";
      sha256 = "0gnmmn1wc09z1q4bb8jkqi2f8vxl26kaa3xrs664q9i651am2mkl";
    })
    { }
}:
with pkgs;
with import (pkgs.path + "/nixos/lib/testing-python.nix") { inherit system; };
rec {
  blockfrost-rustgina = rustPlatform.buildRustPackage {
    name = "blockfrost-icebreakers-api";
    version = "unstable";
    src = ./.;

    buildInputs = [ openssl libiconv rustc binutils pkg-config perl cmake libiconv ];

    cargoLock = {
      lockFile = ./Cargo.lock;
    };

    # skip tests for now
    doCheck = false;

    # Needed to get openssl-sys to use pkgconfig.
    OPENSSL_NO_VENDOR = 1;
    OPENSSL_DIR = "${openssl.dev}";
    OPENSSL_LIB_DIR = "${openssl.out}/lib";

    RUST_BACKTRACE = "full";

  };
}
