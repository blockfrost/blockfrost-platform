{
  inputs,
  targetSystem,
}:
assert builtins.elem targetSystem ["aarch64-linux"]; let
  buildSystem = "x86_64-linux";
  pkgs = inputs.nixpkgs.legacyPackages.${buildSystem};
in rec {
  toolchain = with inputs.fenix.packages.${buildSystem};
    combine [
      minimal.rustc
      minimal.cargo
      targets.aarch64-unknown-linux-gnu.latest.rust-std
    ];

  craneLib = (inputs.crane.mkLib pkgs).overrideToolchain toolchain;

  src = craneLib.cleanCargoSource ../../.;

  pkgsCross = pkgs.pkgsCross.aarch64-multiplatform;

  commonArgs = rec {
    inherit src;
    strictDeps = true;

    CARGO_BUILD_TARGET = "aarch64-unknown-linux-gnu";
    TARGET_CC = "${pkgsCross.stdenv.cc}/bin/${pkgsCross.stdenv.cc.targetPrefix}cc";

    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER = TARGET_CC;

    TESTGEN_HS_PATH = "unused"; # Don’t try to download it in `build.rs`.

    OPENSSL_DIR = "${pkgsCross.openssl.dev}";
    OPENSSL_LIB_DIR = "${pkgsCross.openssl.out}/lib";
    OPENSSL_INCLUDE_DIR = "${pkgsCross.openssl.dev}/include/";

    depsBuildBuild = [
      pkgsCross.stdenv.cc
      #pkgsCross.windows.pthreads
    ];
  };

  # For better caching:
  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

  packageName = (craneLib.crateNameFromCargoToml {cargoToml = src + "/Cargo.toml";}).pname;

  GIT_REVISION = inputs.self.rev or "dirty";

  package = craneLib.buildPackage (commonArgs
    // {
      inherit cargoArtifacts GIT_REVISION;
      doCheck = false;
      postPatch = ''
        sed -r '/^build = .*/d' -i Cargo.toml
        rm build.rs
      '';
    });

  archive = let
    outFileName = "${package.pname}-${package.version}-${inputs.self.shortRev or "dirty"}-${targetSystem}.tar.bz2";
  in
    pkgs.runCommandNoCC "${package.pname}-archive" {} ''
      cp -r ${bundle} ${package.pname}

      mkdir -p $out
      tar -cjvf $out/${outFileName} ${package.pname}/

      # Make it downloadable from Hydra:
      mkdir -p $out/nix-support
      echo "file binary-dist \"$out/${outFileName}\"" >$out/nix-support/hydra-build-products
    '';

  nix-bundle-exe = import inputs.nix-bundle-exe;

  # Portable directory that can be run on any modern Linux:
  bundle =
    (nix-bundle-exe {
      inherit pkgs;
      bin_dir = "bin";
      exe_dir = "exe";
      lib_dir = "lib";
    } "${package}/bin/${packageName}")
    .overrideAttrs (drv: {
      name = packageName;
      buildCommand =
        drv.buildCommand
        + ''
          ( cd $out ; ln -s bin/${packageName} . ; )
        '';
    });
}
