{ pkgs ? import
    (builtins.fetchTarball {
      url = "https://github.com/NixOS/nixpkgs/archive/b83e7f5a04a3acc8e92228b0c4bae68933d504eb.tar.gz";
      sha256 = "1n8x41nizpwid5n3y7jpbi5a6rw0kcc22fyc31bba11m755znccy";
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

    nativebuildInputs = [ rustc binutils pkg-config perl cmake pkgconfig ];

    buildInputs = [ openssl ];

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
