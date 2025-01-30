{
  inputs,
  targetSystem,
}:
assert __elem targetSystem ["x86_64-windows"]; let
  buildSystem = "x86_64-linux";
  pkgs = inputs.nixpkgs.legacyPackages.${buildSystem};
  inherit (pkgs) lib;
in rec {
  toolchain = with inputs.fenix.packages.${buildSystem};
    combine [
      minimal.rustc
      minimal.cargo
      targets.x86_64-pc-windows-gnu.latest.rust-std
    ];

  craneLib = (inputs.crane.mkLib pkgs).overrideToolchain toolchain;

  src = craneLib.cleanCargoSource ../../.;

  pkgsCross = pkgs.pkgsCross.mingwW64;

  commonArgs = {
    inherit src;
    strictDeps = true;

    CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";
    TARGET_CC = "${pkgsCross.stdenv.cc}/bin/${pkgsCross.stdenv.cc.targetPrefix}cc";

    TESTGEN_HS_PATH = "unused"; # Don’t try to download it in `build.rs`.

    OPENSSL_DIR = "${pkgs.openssl.dev}";
    OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
    OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include/";

    depsBuildBuild = [
      pkgsCross.stdenv.cc
      pkgsCross.windows.pthreads
    ];
  };

  # For better caching:
  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

  package = craneLib.buildPackage (commonArgs
    // {
      inherit cargoArtifacts;
      doCheck = false; # we run Windows tests on real Windows on GHA
      postPatch = ''
        sed -r '/^build = .*/d' -i Cargo.toml
        rm build.rs
      '';
    });

  testgen-hs = let
    inherit (inputs.self.internal.x86_64-linux.testgen-hs) version;
  in
    pkgs.fetchzip {
      name = "testgen-hs-${version}";
      url = "https://github.com/input-output-hk/testgen-hs/releases/download/${version}/testgen-hs-${version}-${targetSystem}.zip";
      hash = "sha256-vRUtYueSDu+wSL8iQhlzqtBJBu8vIfBm2c7ynj/bqfU=";
    };

  nsis = import ./windows-nsis.nix {nsisNixpkgs = inputs.nixpkgs-nsis;};

  nsis-plugins = {
    EnVar = pkgs.fetchzip {
      url = "https://nsis.sourceforge.io/mediawiki/images/7/7f/EnVar_plugin.zip";
      hash = "sha256-wuXwwMuRHCKsq/qS2B7IECfJfRCSTC1aHVSrzeP5yuQ=";
      stripRoot = false;
    };
  };

  uninstaller =
    pkgs.runCommandNoCC "uninstaller" {
      buildInputs = [nsis pkgs.wine];
      projectName = package.pname;
      projectVersion = package.version;
      WINEDEBUG = "-all"; # comment out to get normal output (err,fixme), or set to +all for a flood
    } ''
      mkdir home
      export HOME=$(realpath home)
      ln -s ${nsis-plugins.EnVar}/Plugins/x86-unicode EnVar
      substituteAll ${./windows-uninstaller.nsi} uninstaller.nsi
      makensis uninstaller.nsi -V4
      wine tempinstaller.exe /S
      mkdir $out
      mv $HOME/.wine/drive_c/uninstall.exe $out/uninstall.exe
    '';

  installer =
    pkgs.runCommandNoCC "installer" {
      buildInputs = [nsis pkgs.wine];
      projectName = package.pname;
      projectVersion = package.version;
      installerIconPath = "icon.ico";
      lockfileName = "lockfile";
      outFileName = "${package.pname}-${package.version}-${inputs.self.shortRev or "dirty"}-${targetSystem}.exe";
    } ''
      mkdir home
      export HOME=$(realpath home)
      substituteAll ${./windows-installer.nsi} installer.nsi
      cp -r ${bundle} contents
      chmod -R +w contents
      ln -s ${nsis-plugins.EnVar}/Plugins/x86-unicode EnVar
      cp ${uninstaller}/uninstall.exe contents/
      cp ${icon} icon.ico
      makensis installer.nsi -V4
      mkdir $out
      mv "$outFileName" $out/

      # Make it downloadable from Hydra:
      mkdir -p $out/nix-support
      echo "file binary-dist \"$out/$outFileName\"" >$out/nix-support/hydra-build-products
    '';

  bundle = pkgs.runCommandNoCC "bundle" {} ''
    mkdir -p $out
    cp -r ${testgen-hs}/. $out/testgen-hs
    cp -r ${packageWithIcon}/. $out/.
  '';

  archive =
    pkgs.runCommandNoCC "archive" {
      buildInputs = with pkgs; [zip];
      outFileName = "${package.pname}-${package.version}-${inputs.self.shortRev or "dirty"}-${targetSystem}.zip";
    } ''
      cp -r ${bundle} blockfrost-platform
      mkdir -p $out
      zip -q -r $out/$outFileName blockfrost-platform/

      # Make it downloadable from Hydra:
      mkdir -p $out/nix-support
      echo "file binary-dist \"$out/$outFileName\"" >$out/nix-support/hydra-build-products
    '';

  svg2ico = source: let
    sizes = [16 24 32 48 64 128 256 512];
    d2s = d: "${toString d}x${toString d}";
  in
    pkgs.runCommand "${baseNameOf source}.ico" {
      buildInputs = with pkgs; [imagemagick];
    } ''
      ${lib.concatMapStringsSep "\n" (dim: ''
          magick -background none -size ${d2s dim} ${source} ${d2s dim}.png
        '')
        sizes}
      magick ${lib.concatMapStringsSep " " (dim: "${d2s dim}.png") sizes} $out
    '';

  icon = svg2ico (builtins.path {path = ./icon.svg;});

  resource-hacker = pkgs.fetchzip {
    name = "resource-hacker-5.1.7";
    url = "http://www.angusj.com/resourcehacker/resource_hacker.zip";
    hash = "sha256-W5TmyjNNXE3nvn37XYbTM+DBeupPijE4M70LJVKJupU=";
    stripRoot = false;
  };

  packageWithIcon =
    pkgs.runCommand package.name {
      buildInputs = with pkgs; [
        wine
        winetricks
        samba # samba is for bin/ntlm_auth
      ];
      WINEDEBUG = "-all"; # comment out to get normal output (err,fixme), or set to +all for a flood
    } ''
      export HOME=$(realpath $NIX_BUILD_TOP/home)
      mkdir -p $HOME
      ${pkgs.xvfb-run}/bin/xvfb-run \
        --server-args="-screen 0 1920x1080x24 +extension GLX +extension RENDER -ac -noreset" \
        ${pkgs.writeShellScript "wine-setup-inside-xvfb" ''
        set -euo pipefail
        set +e
        wine ${resource-hacker}/ResourceHacker.exe \
          -log res-hack.log \
          -open "$(winepath -w ${package}/bin/*.exe)" \
          -save with-icon.exe \
          -action addoverwrite \
          -res "$(winepath -w ${icon})" \
          -mask ICONGROUP,MAINICON,
        wine_ec="$?"
        set -e
        echo "wine exit code: $wine_ec"
        cat res-hack.log
        if [ "$wine_ec" != 0 ] ; then
          exit "$wine_ec"
        fi
      ''}
      mkdir -p $out
      mv with-icon.exe $out/blockfrost-platform.exe
    '';
}
