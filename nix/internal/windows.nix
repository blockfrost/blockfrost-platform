{
  inputs,
  targetSystem,
}:
assert builtins.elem targetSystem ["x86_64-windows"]; let
  buildSystem = "x86_64-linux";
  pkgs = inputs.nixpkgs.legacyPackages.${buildSystem};
  inherit (pkgs) lib;
  inherit (inputs.self.internal.${buildSystem}) hydraScriptsEnvVars;
in rec {
  toolchain = with inputs.fenix.packages.${buildSystem};
    combine [
      minimal.rustc
      minimal.cargo
      targets.x86_64-pc-windows-gnu.latest.rust-std
    ];

  craneLib = (inputs.crane.mkLib pkgs).overrideToolchain toolchain;

  src = lib.cleanSourceWith {
    src = lib.cleanSource ../../.;
    filter = path: type:
      craneLib.filterCargoSources path type
      || lib.hasSuffix ".sql" path
      || lib.hasSuffix "/LICENSE" path;
    name = "source";
  };

  pkgsCross = pkgs.pkgsCross.mingwW64;

  # Nixpkgs 25.11 restricts mingw_w64-pthreads meta.platforms to Windows only,
  # which breaks cross-compilation from Linux. Override to allow it as a build dep.
  pthreads = pkgsCross.windows.pthreads.overrideAttrs (old: {
    meta = old.meta // {platforms = lib.platforms.all;};
  });

  # Cross-compile libpq for Windows (pkgsCross.postgresql is broken in Nixpkgs):
  libpq-windows = import ./windows-libpq.nix {inherit pkgs pkgsCross pthreads;};

  packageName = craneLib.crateNameFromCargoToml {cargoToml = src + "/crates/platform/Cargo.toml";};

  commonArgs = {
    inherit src;
    inherit (packageName) pname;
    strictDeps = true;

    CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";
    TARGET_CC = "${pkgsCross.stdenv.cc}/bin/${pkgsCross.stdenv.cc.targetPrefix}cc";

    TESTGEN_HS_PATH = "unused"; # Don’t try to download it in `build.rs`.

    OPENSSL_DIR = "${pkgs.openssl.dev}";
    OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
    OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include/";

    PQ_LIB_DIR = "${libpq-windows}/lib";
    PQ_LIB_STATIC = "1";

    depsBuildBuild = [
      pkgsCross.stdenv.cc
      pthreads
    ];
  };

  # For better caching:
  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

  GIT_REVISION = inputs.self.rev or "dirty";

  blockfrost-platform = craneLib.buildPackage (commonArgs
    // {
      inherit cargoArtifacts GIT_REVISION;
      doCheck = false; # we run Windows tests on real Windows on GHA
      postPatch = ''
        find -name 'Cargo.toml' | while IFS= read -r cargo_toml ; do
          sed -r '/^build = .*/d' -i "$cargo_toml"
        done
        find -name 'build.rs' -delete
      '';
    });

  gatewayCargoToml = builtins.fromTOML (builtins.readFile (builtins.path {path = src + "/crates/gateway/Cargo.toml";}));
  sdkBridgeCargoToml = builtins.fromTOML (builtins.readFile (builtins.path {path = src + "/crates/sdk_bridge/Cargo.toml";}));
  blockfrost-gateway = craneLib.buildPackage (commonArgs
    // {
      inherit cargoArtifacts GIT_REVISION;
      pname = gatewayCargoToml.package.name;
      doCheck = false; # we run Windows tests on real Windows on GHA
      cargoExtraArgs = "--package blockfrost-gateway";
      postPatch = ''
        find -name 'Cargo.toml' | while IFS= read -r cargo_toml ; do
          sed -r '/^build = .*/d' -i "$cargo_toml"
        done
        find -name 'build.rs' -delete
      '';
    }
    // (builtins.listToAttrs hydraScriptsEnvVars));

  blockfrost-sdk-bridge = craneLib.buildPackage (commonArgs
    // {
      inherit cargoArtifacts GIT_REVISION;
      pname = sdkBridgeCargoToml.package.name;
      doCheck = false; # we run Windows tests on real Windows on GHA
      cargoExtraArgs = "--package blockfrost-sdk-bridge";
      postPatch = ''
        find -name 'Cargo.toml' | while IFS= read -r cargo_toml ; do
          sed -r '/^build = .*/d' -i "$cargo_toml"
        done
        find -name 'build.rs' -delete
      '';
    });

  testgen-hs = let
    inherit (inputs.self.internal.x86_64-linux.testgen-hs) version;
  in
    pkgs.fetchzip {
      name = "testgen-hs-${version}";
      url = "https://github.com/blockfrost/testgen-hs/releases/download/${version}/testgen-hs-${version}-${targetSystem}.zip";
      hash = "sha256-LXE1RBKgal1Twh7j2hpCfNLsBMEcqSwGHb4bj/Imd9Q=";
    };

  nsis-plugins = {
    EnVar = pkgs.fetchzip {
      url = "https://nsis.sourceforge.io/mediawiki/images/7/7f/EnVar_plugin.zip";
      hash = "sha256-wuXwwMuRHCKsq/qS2B7IECfJfRCSTC1aHVSrzeP5yuQ=";
      stripRoot = false;
    };
  };

  uninstaller =
    pkgs.runCommand "uninstaller"
    {
      buildInputs = [pkgs.nsis pkgs.wine];
      projectName = blockfrost-platform.pname;
      projectVersion = blockfrost-platform.version;
      WINEDEBUG = "-all"; # comment out to get normal output (err,fixme), or set to +all for a flood
      WINEDLLOVERRIDES = "mscoree,mshtml="; # don't ask about Mono or Gecko
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

  make-signed-installer = make-installer {doSign = true;};

  installer = unsigned-installer;

  unsigned-installer = pkgs.stdenv.mkDerivation {
    name = "unsigned-installer";
    dontUnpack = true;
    buildPhase = ''
      ${make-installer {doSign = false;}}/bin/* | tee make-installer.log
    '';
    installPhase = ''
      mkdir -p $out
      cp $(tail -n 1 make-installer.log) $out/

      # Make it downloadable from Hydra:
      mkdir -p $out/nix-support
      echo "file binary-dist \"$(echo $out/*.exe)\"" >$out/nix-support/hydra-build-products
    '';
  };

  make-installer = {doSign ? false}: let
    outFileName = "${blockfrost-platform.pname}-${blockfrost-platform.version}-${inputs.self.shortRev or "dirty"}-${targetSystem}.exe";
    installer-nsi =
      pkgs.runCommand "installer.nsi"
      {
        inherit outFileName;
        projectName = blockfrost-platform.pname;
        projectVersion = blockfrost-platform.version;
        installerIconPath = "icon.ico";
        lockfileName = "lockfile";
      } ''
        substituteAll ${./windows-installer.nsi} $out
      '';
  in
    pkgs.writeShellApplication {
      name = "pack-and-sign";
      runtimeInputs = with pkgs; [bash coreutils pkgs.nsis];
      runtimeEnv = {
        inherit outFileName;
      };
      text = ''
        set -euo pipefail
        workDir=$(mktemp -d)
        cd "$workDir"

        ${
          if doSign
          then ''
            sign_cmd() {
              echo "Signing: ‘$1’…"
              ssh HSM <"$1" >"$1".signed
              mv "$1".signed "$1"
            }
          ''
          else ''
            sign_cmd() {
              echo "Would sign: ‘$1’"
            }
          ''
        }

        cp ${installer-nsi} installer.nsi
        cp -r ${bundle} contents
        chmod -R +w contents
        ln -s ${nsis-plugins.EnVar}/Plugins/x86-unicode EnVar
        cp ${uninstaller}/uninstall.exe contents/
        cp ${icon} icon.ico

        chmod -R +w contents
        find contents '(' -iname '*.exe' -o -iname '*.dll' ')' | sort | while IFS= read -r binary_to_sign ; do
          sign_cmd "$binary_to_sign"
        done

        makensis installer.nsi -V4

        sign_cmd "$outFileName"

        echo
        echo "Done, you can upload it to GitHub releases:"
        echo "$workDir"/"$outFileName"
      '';
    };

  # XXX: there’s no Hydra build for Windows currently, as `hydra-cluster`
  # depends on the `unix` package, see <https://github.com/cardano-scaling/hydra/issues/2360>.
  bundle =
    pkgs.runCommand "bundle" {
      buildInputs = [pkgs.wine64];
      WINEDEBUG = "-all";
      WINEDLLOVERRIDES = "mscoree,mshtml="; # don't ask about Mono or Gecko
    } ''
      mkdir home
      export HOME=$(realpath home)
      mkdir -p $out
      cp -r ${packageWithIcon}/. $out/.
      cp -r ${dolos}/bin/. $out/.
      wine64 $out/${packageName.pname}.exe --version
    '';

  archive =
    pkgs.runCommand "archive"
    {
      buildInputs = with pkgs; [zip];
      outFileName = "${blockfrost-platform.pname}-${blockfrost-platform.version}-${inputs.self.shortRev or "dirty"}-${targetSystem}.zip";
    } ''
      cp -r ${bundle} ${packageName.pname}
      mkdir -p $out
      zip -q -r $out/$outFileName ${packageName.pname}/

      # Make it downloadable from Hydra:
      mkdir -p $out/nix-support
      echo "file binary-dist \"$out/$outFileName\"" >$out/nix-support/hydra-build-products
    '';

  svg2ico = source: let
    sizes = [16 24 32 48 64 128 256 512];
    d2s = d: "${toString d}x${toString d}";
  in
    pkgs.runCommand "${baseNameOf source}.ico"
    {
      buildInputs = with pkgs; [imagemagick];
    } ''
      ${lib.concatMapStringsSep "\n" (dim: ''
          magick -background none ${source} -resize ${d2s dim} ${d2s dim}.png
        '')
        sizes}
      magick ${lib.concatMapStringsSep " " (dim: "${d2s dim}.png") sizes} $out
    '';

  icon = svg2ico (builtins.path {path = ./icon.svg;});

  resource-hacker = pkgs.fetchzip {
    name = "resource-hacker-5.1.7";
    url = "http://www.angusj.com/resourcehacker/resource_hacker.zip";
    hash = "sha256-PUY1e5DtfqYiwcYol+JTkCXu5Al++WQONnTFxcN6Ass=";
    stripRoot = false;
  };

  # FIXME: Dolos v1.0.0-rc.12 depends on a fjall branch that was deleted after merge:
  # https://github.com/fjall-rs/fjall/pull/259
  # Patch the source to use the pinned commit rev instead of the defunct branch name.
  dolosSrc = pkgs.runCommand "dolos-src-patched" {} ''
    cp -r ${inputs.dolos} $out
    chmod -R +w $out
    sed -i 's|branch = "recovery/change-flush-queueing"|rev = "2443c7bcf6f53920efef836518d76e865974c4ca"|' $out/Cargo.toml
    sed -i 's|branch=recovery%2Fchange-flush-queueing|rev=2443c7bcf6f53920efef836518d76e865974c4ca|g' $out/Cargo.lock
  '';

  dolos = craneLib.buildPackage {
    src = dolosSrc;
    GIT_REVISION = inputs.dolos.rev;
    strictDeps = true;

    CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";
    TARGET_CC = "${pkgsCross.stdenv.cc}/bin/${pkgsCross.stdenv.cc.targetPrefix}cc";

    TESTGEN_HS_PATH = "unused"; # Don’t try to download it in `build.rs`.

    OPENSSL_DIR = "${pkgs.openssl.dev}";
    OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
    OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include/";

    depsBuildBuild = [
      pkgsCross.stdenv.cc
      pthreads
    ];

    doCheck = false; # we run Windows tests on real Windows on GHA
  };

  packageWithIcon =
    pkgs.runCommand blockfrost-platform.name
    {
      buildInputs = with pkgs; [
        wine
        winetricks
        samba # samba is for bin/ntlm_auth
      ];
      WINEDEBUG = "-all"; # comment out to get normal output (err,fixme), or set to +all for a flood
      WINEDLLOVERRIDES = "mscoree,mshtml="; # don't ask about Mono or Gecko
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
          -open "$(winepath -w ${blockfrost-platform}/bin/*.exe)" \
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
      mv with-icon.exe $out/${packageName.pname}.exe
    '';
}
