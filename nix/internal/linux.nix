{
  inputs,
  targetSystem,
  unix,
}:
assert builtins.elem targetSystem ["x86_64-linux" "aarch64-linux"]; let
  buildSystem = targetSystem;
  pkgs = inputs.nixpkgs.legacyPackages.${buildSystem};
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
      } "${unix.package}/libexec/${unix.packageName}")
      .overrideAttrs (drv: {
        name = unix.packageName;
        buildCommand =
          drv.buildCommand
          + ''
            chmod -R +w $out
            ${with pkgs; lib.getExe rsync} -a ${bundle-dolos}/. $out/.
            ${with pkgs; lib.getExe rsync} -a ${bundle-hydra}/. $out/.
            chmod -R +w $out
            ( cd $out ; ln -s bin/{${unix.packageName},dolos,hydra-node} ./ ; )
          '';
      });

    bundle-dolos = nix-bundle-exe {
      inherit pkgs;
      bin_dir = "bin";
      exe_dir = "exe";
      lib_dir = "lib";
    } "${unix.dolos}/bin/dolos";

    bundle-hydra = nix-bundle-exe {
      inherit pkgs;
      bin_dir = "bin";
      exe_dir = "exe";
      lib_dir = "lib";
    } "${unix.hydra-node}/bin/hydra-node";
  }
