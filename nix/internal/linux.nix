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
      outFileName = "${unix.blockfrost-platform.pname}-${unix.blockfrost-platform.version}-${inputs.self.shortRev or "dirty"}-${targetSystem}.tar.bz2";
    in
      pkgs.runCommandNoCC "${unix.blockfrost-platform.pname}-archive" {} ''
        cp -r ${bundle} ${unix.blockfrost-platform.pname}

        mkdir -p $out
        tar -cjvf $out/${outFileName} ${unix.blockfrost-platform.pname}/

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
      } "${unix.blockfrost-platform}/libexec/${unix.packageName.pname}")
      .overrideAttrs (drv: {
        name = unix.packageName.pname;
        buildCommand =
          drv.buildCommand
          + ''
            chmod -R +w $out
            ${with pkgs; lib.getExe rsync} -a ${bundle-dolos}/. $out/.
            ${with pkgs; lib.getExe rsync} -a ${bundle-hydra}/. $out/.
            chmod -R +w $out
            ( cd $out ; ln -s bin/{${unix.packageName.pname},dolos,hydra-node} ./ ; )
            $out/bin/${unix.packageName.pname} --version
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

    # Portable directory that can be run on any modern Linux:
    bundle-bridge =
      (nix-bundle-exe {
        inherit pkgs;
        bin_dir = "bin";
        exe_dir = "exe";
        lib_dir = "lib";
      } "${unix.blockfrost-sdk-bridge}/libexec/${unix.sdkBridgeCargoToml.package.name}")
      .overrideAttrs (drv: {
        inherit (unix.sdkBridgeCargoToml.package) name;
        buildCommand =
          drv.buildCommand
          + ''
            chmod -R +w $out
            ${with pkgs; lib.getExe rsync} -a ${bundle-hydra}/. $out/.
            chmod -R +w $out
            ( cd $out ; ln -s bin/{${unix.sdkBridgeCargoToml.package.name},hydra-node} ./ ; )
            $out/bin/${unix.sdkBridgeCargoToml.package.name} --version
          '';
      });

    archive-bridge = let
      outFileName = "${unix.sdkBridgeCargoToml.package.name}-${unix.blockfrost-platform.version}-${inputs.self.shortRev or "dirty"}-${targetSystem}.tar.bz2";
    in
      pkgs.runCommandNoCC "${unix.sdkBridgeCargoToml.package.name}-archive" {} ''
        cp -r ${bundle-bridge} ${unix.sdkBridgeCargoToml.package.name}

        mkdir -p $out
        tar -cjvf $out/${outFileName} ${unix.sdkBridgeCargoToml.package.name}/

        # Make it downloadable from Hydra:
        mkdir -p $out/nix-support
        echo "file binary-dist \"$out/${outFileName}\"" >$out/nix-support/hydra-build-products
      '';
  }
