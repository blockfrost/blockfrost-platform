{
  inputs,
  targetSystem,
  unix,
}:
assert __elem targetSystem ["x86_64-linux" "aarch64-linux"]; let
  buildSystem = targetSystem;
  pkgs = inputs.nixpkgs.legacyPackages.${buildSystem};
  inherit (pkgs) lib;
in
  unix
  // rec {
    archive = let
      outFileName = "${unix.package.pname}-${unix.package.version}-${inputs.self.shortRev or "dirty"}-${targetSystem}.tar.bz2";
    in
      pkgs.runCommandNoCC "${unix.package.pname}-archive" {} ''
        cp -r ${bundle} ${unix.package.pname}

        mkdir -p $out
        tar -cjvf $out/${outFileName} ${unix.package.pname}/

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
      } "${unix.package}/libexec/blockfrost-platform")
      .overrideAttrs (drv: {
        name = "blockfrost-platform";
        buildCommand =
          drv.buildCommand
          + ''
            mkdir -p $out/lib/testgen-hs
            cp ${lib.getExe unix.testgen-hs} $out/lib/testgen-hs/
            ( cd $out ; ln -s bin/blockfrost-platform . ; )
          '';
      });
  }
