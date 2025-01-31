{
  inputs,
  targetSystem,
  unix,
}:
assert __elem targetSystem ["x86_64-darwin" "aarch64-darwin"]; let
  buildSystem = targetSystem;
  pkgs = inputs.nixpkgs.legacyPackages.${buildSystem};
  inherit (pkgs) lib;
in
  unix
  // rec {
    archive = let
      outFileName = "${unix.package.pname}-${unix.package.version}-${inputs.self.shortRev or "dirty"}-${targetSystem}.tar.bz2";
    in
      pkgs.runCommandNoCC "${unix.package.pname}-archive" {
        passthru = {inherit outFileName;};
      } ''
        cp -r ${bundle} ${unix.package.pname}

        mkdir -p $out
        tar -cjvf $out/${outFileName} ${unix.package.pname}/

        # Make it downloadable from Hydra:
        mkdir -p $out/nix-support
        echo "file binary-dist \"$out/${outFileName}\"" >$out/nix-support/hydra-build-products
      '';

    nix-bundle-exe-lib-subdir = let
      patched = pkgs.runCommand "nix-bundle-exe-same-dir" {} ''
        cp -R ${inputs.nix-bundle-exe} $out
        chmod -R +w $out
        sed -r 's+@executable_path/\$relative_bin_to_lib/\$lib_dir+@executable_path/lib+g' -i $out/bundle-macos.sh
      '';
    in
      import patched {
        inherit pkgs;
        bin_dir = ".";
        lib_dir = "./lib";
      };

    # Portable directory that can be run on any modern Linux:
    bundle =
      (nix-bundle-exe-lib-subdir "${unix.package}/libexec/blockfrost-platform")
      .overrideAttrs (drv: {
        name = "blockfrost-platform";
        buildCommand =
          drv.buildCommand
          + ''
            mkdir -p $out/libexec
            mv $out/{blockfrost-platform,lib} $out/libexec
            mkdir -p $out/bin
            ( cd $out/bin ; ln -s ../libexec/blockfrost-platform ./ ; )
            cp -r ${bundle-testgen-hs} $out/libexec/testgen-hs
          '';
      });

    bundle-testgen-hs = nix-bundle-exe-lib-subdir (lib.getExe unix.testgen-hs);

    # Contents of the <https://github.com/blockfrost/homebrew-tap>
    # repo. We replace that workdir on each release.
    homebrew-tap =
      pkgs.runCommandNoCC "homebrew-repo" {
        version = unix.package.version;
        url_x86_64 = "${unix.releaseBaseUrl}/${inputs.self.internal.x86_64-darwin.archive.outFileName}";
        url_aarch64 = "${unix.releaseBaseUrl}/${inputs.self.internal.aarch64-darwin.archive.outFileName}";
      } ''
        cp -r ${./homebrew-tap} $out
        chmod -R +w $out

        sha256_x86_64=$(sha256sum ${inputs.self.internal.x86_64-darwin.archive}/*.tar.bz2 | cut -d' ' -f1)
        export sha256_x86_64
        sha256_aarch64=$(sha256sum ${inputs.self.internal.aarch64-darwin.archive}/*.tar.bz2 | cut -d' ' -f1)
        export sha256_aarch64

        substituteAllInPlace $out/Formula/blockfrost-platform.rb
      '';
  }
