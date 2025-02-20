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

    # Portable directory that can be run on any modern Darwin:
    bundle =
      (nix-bundle-exe-lib-subdir "${unix.package}/libexec/${unix.packageName}")
      .overrideAttrs (drv: {
        name = unix.packageName;
        buildCommand =
          drv.buildCommand
          + ''
            mkdir -p $out/libexec
            mv $out/{${unix.packageName},lib} $out/libexec
            mkdir -p $out/bin
            ( cd $out/bin ; ln -s ../libexec/${unix.packageName} ./ ; )
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

    # `CFBundleExecutable` has to be a Mach-O executable, but we can simply launch a Bash script from there:
    dmg-launcher = pkgs.runCommand "dmg-launcher" {
      buildInputs = with pkgs; [rustc clang darwin.cctools darwin.binutils];
      src = ''
        use std::os::unix::process::CommandExt;
        use std::process::Command;
        fn main() {
            let exe = std::env::current_exe().expect("failed to read `std::env::current_exe`");
            let resolved = std::fs::canonicalize(exe).expect("failed to canonicalize");
            let script = format!("{}.sh", resolved.to_string_lossy());
            let argv = std::env::args().skip(1);
            let error = Command::new(&script).args(argv).exec();
            panic!("failed to exec {}: {}", script, error.to_string())
        }
      '';
    } ''rustc - <<<"$src" && mv rust_out $out'';

    prettyName = "Blockfrost Platform";

    app-bundle =
      pkgs.runCommandNoCC "app-bundle" rec {
        buildInputs = with pkgs; [shellcheck];
        appName = prettyName;
        launcherName = "BlockfrostPlatform";
        infoPlist = pkgs.writeText "Info.plist" ''
          <?xml version="1.0" encoding="UTF-8"?>
          <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
          <plist version="1.0">
          <dict>
              <key>CFBundleDevelopmentRegion</key>
              <string>en</string>
              <key>CFBundleExecutable</key>
              <string>${launcherName}</string>
              <key>CFBundleIdentifier</key>
              <string>io.blockfrost.platform</string>
              <key>CFBundleName</key>
              <string>${appName}</string>
              <key>CFBundleDisplayName</key>
              <string>${appName}</string>
              <key>CFBundleVersion</key>
              <string>${unix.package.version}-${inputs.self.shortRev or "dirty"}</string>
              <key>CFBundleShortVersionString</key>
              <string>${unix.package.version}</string>
              <key>CFBundleIconFile</key>
              <string>iconset</string>
              <key>LSMinimumSystemVersion</key>
              <string>10.14</string>
              <key>NSHighResolutionCapable</key>
              <string>True</string>
              <!-- avoid showing the app on the Dock -->
              <key>LSUIElement</key>
              <string>1</string>
          </dict>
          </plist>
        '';
      } ''
        app=$out/Applications/"$appName".app/Contents
        macos="$app"/MacOS
        resources="$app"/Resources
        mkdir -p "$app"/MacOS "$app"/Resources
        cp $infoPlist "$app"/Info.plist
        cp ${dmg-launcher} "$app"/MacOS/"$launcherName"
        cp ${./darwin-launcher.sh} "$app"/MacOS/"$launcherName".sh
        cp ${./darwin-terminal-init.sh} "$app"/MacOS/darwin-terminal-init.sh
        chmod +x "$app"/MacOS/"$launcherName"*
        shellcheck "$app"/MacOS/"$launcherName".sh
        shellcheck "$app"/MacOS/darwin-terminal-init.sh
        cp -r ${bundle} "$app"/MacOS/bundle
        cp -r ${iconset} "$app"/Resources/iconset.icns
      '';

    iconset = svg2icns ./icon.svg;

    svg2icns = source: let
      sizes = [16 18 19 22 24 32 40 48 64 128 256 512 1024];
      d2s = d: "${toString d}x${toString d}";
    in
      pkgs.runCommand "${baseNameOf source}.icns" {
        buildInputs = with pkgs; [imagemagick];
      } ''
        mkdir -p iconset.iconset
        ${lib.concatMapStringsSep "\n" (dim: ''
            magick -background none -size ${d2s dim}       ${source} iconset.iconset/icon_${d2s dim}.png
            magick -background none -size ${d2s (dim * 2)} ${source} iconset.iconset/icon_${d2s dim}@2x.png
          '')
          sizes}
        /usr/bin/iconutil --convert icns --output $out iconset.iconset
      '';

    installer = unsigned-dmg;

    # See <https://dmgbuild.readthedocs.io/en/latest/settings.html>:
    dmgbuildSettingsPy = let
      s = lib.escapeShellArg;
    in
      pkgs.writeText "settings.py" ''
        import os.path

        app_path = defines.get("app_path", "/non-existent.app")
        icon_path = defines.get("icon_path", "/non-existent.icns")
        app_name = os.path.basename(app_path)

        # UDBZ (bzip2) is 154 MiB, while UDZO (gzip) is 204 MiB
        format = "UDBZ"
        size = None
        files = [app_path]
        symlinks = {"Applications": "/Applications"}
        hide_extension = [ app_name ]

        icon = icon_path

        icon_locations = {app_name: (140, 120), "Applications": (500, 120)}
        background = "builtin-arrow"

        show_status_bar = False
        show_tab_view = False
        show_toolbar = False
        show_pathbar = False
        show_sidebar = False
        sidebar_width = 180

        window_rect = ((200, 200), (640, 320))
        default_view = "icon-view"
        show_icon_preview = False

        include_icon_view_settings = "auto"
        include_list_view_settings = "auto"

        arrange_by = None
        grid_offset = (0, 0)
        grid_spacing = 100
        scroll_position = (0, 0)
        label_pos = "bottom"  # or 'right'
        text_size = 16
        icon_size = 128

        # license = { … }
      '';

    # XXX: this needs to be `nix run` on `iog-mac-studio-arm-2-signing` or a similar machine.
    # It can’t be a pure derivation because it needs to impurely access the Apple signing machinery.
    make-signed-dmg = make-dmg {doSign = true;};

    unsigned-dmg = pkgs.stdenv.mkDerivation {
      name = "dmg-image";
      dontUnpack = true;
      buildPhase = ''
        ${make-dmg {doSign = false;}}/bin/* | tee make-installer.log
      '';
      installPhase = ''
        mkdir -p $out
        cp $(tail -n 1 make-installer.log) $out/

        # Make it downloadable from Hydra:
        mkdir -p $out/nix-support
        echo "file binary-dist \"$(echo $out/*.dmg)\"" >$out/nix-support/hydra-build-products
      '';
    };

    make-dmg = {doSign ? false}: let
      outFileName = "${unix.package.pname}-${unix.package.version}-${inputs.self.shortRev or "dirty"}-${targetSystem}.dmg";
      credentials = "/var/lib/buildkite-agent/signing.sh";
      codeSigningConfig = "/var/lib/buildkite-agent/code-signing-config.json";
      signingConfig = "/var/lib/buildkite-agent/signing-config.json";
      packAndSign = pkgs.writeShellApplication {
        name = "pack-and-sign";
        runtimeInputs = with pkgs; [bash coreutils jq];
        text = ''
          set -euo pipefail

          ${
            if doSign
            then ''
              codeSigningIdentity=$(jq -r .codeSigningIdentity ${codeSigningConfig})
              codeSigningKeyChain=$(jq -r .codeSigningKeyChain ${codeSigningConfig})
              # unused: signingIdentity=$(jq -r .signingIdentity ${signingConfig})
              # unused: signingKeyChain=$(jq -r .signingKeyChain ${signingConfig})

              echo "Checking if notarization credentials are defined..."
              if [ -z "''${NOTARY_USER:-}" ] || [ -z "''${NOTARY_PASSWORD:-}" ] || [ -z "''${NOTARY_TEAM_ID:-}" ] ; then
                echo >&2 "Fatal: please set \$NOTARY_USER, \$NOTARY_PASSWORD, and \$NOTARY_TEAM_ID"
                exit 1
              fi
            ''
            else ''
              echo >&2 "Warning: the DMG will be unsigned"
            ''
          }

          workDir=$(mktemp -d)
          appName=${lib.escapeShellArg prettyName}.app
          appDir=${app-bundle}/Applications/"$appName"

          echo "Info: workDir = $workDir"
          cd "$workDir"

          echo "Copying..."
          cp -r "$appDir" ./.
          chmod -R +w .

          bundlePath="$workDir/$appName"

          ${
            if doSign
            then ''
              echo
              echo "Signing code..."

              # Ensure the code signing identity is found and set the keychain search path:
              security show-keychain-info "$codeSigningKeyChain"
              security find-identity -v -p codesigning "$codeSigningKeyChain"
              security list-keychains -d user -s "$codeSigningKeyChain"

              # Sign the whole component deeply
              codesign \
                --force --verbose=4 --deep --strict --timestamp --options=runtime \
                --entitlements ${./darwin-entitlements.xml} \
                --sign "$codeSigningIdentity" \
                "$bundlePath"

              # Verify the signing
              codesign --verbose=4 --verify --deep --strict "$bundlePath"
              codesign --verbose=4 --verify --deep --strict --display -r- "$bundlePath"
              codesign -d --entitlements :- "$bundlePath"
            ''
            else ""
          }

          echo
          echo "Making the DMG..."
          ${dmgbuild}/bin/dmgbuild \
            -D app_path="$bundlePath" \
            -D icon_path=${badgeIcon} \
            -s ${dmgbuildSettingsPy} \
            ${lib.escapeShellArg prettyName} ${outFileName}

          ${
            if doSign
            then ''
              # FIXME: this doesn’t work outside of `buildkite-agent`, it seems:
              #(
              #  source ${credentials}
              #  security unlock-keychain -p "$SIGNING" "$signingKeyChain"
              #)

              echo
              echo "Signing the DMG..."
              codesign \
                --force --verbose=4 --timestamp --options=runtime \
                --sign "$codeSigningIdentity" \
                ${outFileName}

              echo
              echo "Submitting for notarization..."
              xcrun notarytool submit \
                --apple-id "$NOTARY_USER" \
                --password "$NOTARY_PASSWORD" \
                --team-id "$NOTARY_TEAM_ID" \
                --wait ${outFileName}

              echo
              echo "Stapling the notarization ticket..."
              xcrun stapler staple ${outFileName}
            ''
            else ""
          }

          echo
          echo "Done, you can upload it to GitHub releases:"
          echo "$workDir"/${outFileName}
        '';
      };
    in
      pkgs.writeShellApplication {
        name = "make-dmg";
        runtimeInputs = with pkgs; [bash coreutils jq];
        text = ''
          set -euo pipefail
          cd /
          ${
            if doSign
            then ''
              exec sudo -u buildkite-agent \
                "NOTARY_USER=''${NOTARY_USER:-}" \
                "NOTARY_PASSWORD=''${NOTARY_PASSWORD:-}" \
                "NOTARY_TEAM_ID=''${NOTARY_TEAM_ID:-}" \
                ${lib.getExe packAndSign}
            ''
            else ''
              exec ${lib.getExe packAndSign}
            ''
          }
        '';
      };

    pythonPackages = pkgs.python3Packages;

    mac_alias = pythonPackages.buildPythonPackage rec {
      pname = "mac_alias";
      version = "2.2.2-rc1";
      src = pkgs.fetchFromGitHub {
        owner = "dmgbuild";
        repo = pname;
        rev = "c5c6fa8f59792a6e1b3812086e540857ef31be45";
        hash = "sha256-5s4aGzDIDJ4XSlSVDcjf5Eujzj7eDv6vK8iS1GXcpkc=";
      };
      propagatedBuildInputs = with pythonPackages; [setuptools];
      format = "pyproject";
      postFixup = ''rm -r $out/bin''; # no __main__.py
    };

    ds_store = pythonPackages.buildPythonPackage rec {
      pname = "ds_store";
      version = "1.3.1";
      src = pkgs.fetchFromGitHub {
        owner = "dmgbuild";
        repo = pname;
        rev = "v${version}";
        hash = "sha256-45lmkE61uXVCBUMyVVzowTJoALY1m9JI68s7Yb0vCks=";
      };
      propagatedBuildInputs = (with pythonPackages; [setuptools]) ++ [mac_alias];
      format = "pyproject";
      postFixup = ''sed -r 's+main\(\)+main(sys.argv[1:])+g' -i $out/bin/.${pname}-wrapped'';
    };

    pyobjc = rec {
      version = "9.2";

      commonPreBuild = ''
        # 1004 instead of 10.4, 1100 instead of 11.0 etc.
        PyObjC_BUILD_RELEASE=$(echo "$MACOSX_DEPLOYMENT_TARGET" | awk -F. '{printf "%02d%02d\n", $1, $2}')

        # Force it to target our ‘darwinMinVersion’, it’s not recognized correctly:
        grep -RF -- '-DPyObjC_BUILD_RELEASE=%02d%02d' | cut -d: -f1 | while IFS= read -r file ; do
          sed -r '/-DPyObjC_BUILD_RELEASE=%02d%02d/{s/%02d%02d/'"$PyObjC_BUILD_RELEASE"'/;n;d;}' -i "$file"
        done

        # impurities:
        ( grep -RF '/usr/bin/xcrun' || true ; ) | cut -d: -f1 | while IFS= read -r file ; do
          sed -r "s+/usr/bin/xcrun+$(${lib.getExe pkgs.which} xcrun)+g" -i "$file"
        done
        ( grep -RF '/usr/bin/python' || true ; ) | cut -d: -f1 | while IFS= read -r file ; do
          sed -r "s+/usr/bin/python+$(${lib.getExe pkgs.which} python)+g" -i "$file"
        done
      '';

      core = pythonPackages.buildPythonPackage rec {
        pname = "pyobjc-core";
        inherit version;
        src = pythonPackages.fetchPypi {
          inherit pname version;
          hash = "sha256-1zS5KR/skf9OOuOLnGg53r8Ct5wHMUR26H2o6QssaMM=";
        };
        buildInputs = with pkgs; [apple-sdk_11 (darwinMinVersionHook "11.0") darwin.libffi];
        hardeningDisable = ["strictoverflow"]; # -fno-strict-overflow is not supported in clang on darwin
        NIX_CFLAGS_COMPILE = ["-Wno-error=deprecated-declarations" "-Wno-error=cast-of-sel-type"];
        preBuild =
          commonPreBuild
          + ''
            sed -r 's+/usr/include/ffi+${pkgs.darwin.libffi.dev}/include+g' -i setup.py

            # Turn off clang’s Link Time Optimization, or else we can’t recognize (and link) Objective C .o’s:
            sed -r 's/"-flto=[^"]+",//g' -i setup.py

            # Fix some test code:
            grep -RF '"sw_vers"' | cut -d: -f1 | while IFS= read -r file ; do
              sed -r "s+"sw_vers"+"/usr/bin/sw_vers"+g" -i "$file"
            done
          '';
        # XXX: We’re turning tests off, because they’re mostly working (0.54% failures among 4,600 tests),
        # and I don’t have any more time to investigate now (maybe in a Nixpkgs contribution in the future):
        #
        # pyobjc-core> Ran 4600 tests in 273.830s
        # pyobjc-core> FAILED (failures=3, errors=25, skipped=4, expected failures=3, unexpected successes=1)
        # pyobjc-core> SUMMARY: {'count': 4600, 'fails': 3, 'errors': 25, 'xfails': 3, 'xpass': 0, 'skip': 4}
        # pyobjc-core> error: some tests failed
        dontUseSetuptoolsCheck = true;
        doCheck = false;
      };

      framework-Cocoa = pythonPackages.buildPythonPackage rec {
        pname = "pyobjc-framework-Cocoa";
        inherit version;
        src = pythonPackages.fetchPypi {
          inherit pname version;
          hash = "sha256-79eAgIctjI3mwrl+Dk6smdYgOl0WN6oTXQcdRk6y21M=";
        };
        buildInputs = with pkgs; [apple-sdk_11 (darwinMinVersionHook "11.0")];
        propagatedBuildInputs = [core];
        hardeningDisable = ["strictoverflow"]; # -fno-strict-overflow is not supported in clang on darwin
        preBuild = commonPreBuild;
        dontUseSetuptoolsCheck = true; # XXX: majority is passing
      };

      framework-Quartz = pythonPackages.buildPythonPackage rec {
        pname = "pyobjc-framework-Quartz";
        inherit version;
        src = pythonPackages.fetchPypi {
          inherit pname version;
          hash = "sha256-9YYYO5ue9/Fl8ERKe3FO2WXXn26SYXyq+GkVDc/Vpys=";
        };
        buildInputs = with pkgs; [apple-sdk_11 (darwinMinVersionHook "11.0")];
        propagatedBuildInputs = [framework-Cocoa];
        hardeningDisable = ["strictoverflow"]; # -fno-strict-overflow is not supported in clang on darwin
        preBuild = commonPreBuild;
        dontUseSetuptoolsCheck = true; # XXX: majority is passing
      };
    };

    CLTools_Executables = let
      mkSusDerivation = args:
        pkgs.stdenvNoCC.mkDerivation (args
          // {
            dontBuild = true;
            darwinDontCodeSign = true;
            nativeBuildInputs = with pkgs; [cpio pbzx];
            outputs = ["out"];
            unpackPhase = ''
              pbzx $src | cpio -idm
            '';
            passthru = {
              inherit (args) version;
            };
          });
    in
      mkSusDerivation {
        pname = "CLTools_Executables";
        version = "11.0.0";
        src = pkgs.fetchurl {
          url = "http://swcdn.apple.com/content/downloads/46/21/001-89745-A_56FM390IW5/v1um2qppgfdnam2e9cdqcqu2r6k8aa3lis/CLTools_Executables.pkg";
          sha256 = "0nvb1qx7l81l2wcl8wvgbpsg5rcn51ylhivqmlfr2hrrv3zrrpl0";
        };
        installPhase = ''
          mv Library/Developer/CommandLineTools $out
        '';
      };

    # How to get it in a saner way?
    SetFile =
      pkgs.runCommand "SetFile" {
        meta.mainProgram = "SetFile";
      } ''
        mkdir -p $out/bin
        cp ${CLTools_Executables}/usr/bin/SetFile $out/bin/
      '';

    # dmgbuild doesn’t rely on Finder to customize appearance of the mounted DMT directory
    # Finder is unreliable and requires graphical environment
    # dmgbuild still uses /usr/bin/hdiutil, but it's possible to use it w/o root (in 2 stages), which they do
    dmgbuild = pythonPackages.buildPythonPackage rec {
      pname = "dmgbuild";
      version = "1.6.1-rc1";
      src = pkgs.fetchFromGitHub {
        owner = "dmgbuild";
        repo = pname;
        rev = "cdf7ba052fcd09f60132af183ce2b1388566cc75";
        hash = "sha256-QkVEECnUmEROZNzczKHLYTjSyoLz3V8v2uhuJWntgog=";
      };
      patches = [./dmgbuild--force-badge.diff];
      buildInputs = with pkgs; [apple-sdk_11 (darwinMinVersionHook "11.0")];
      propagatedBuildInputs = (with pythonPackages; [setuptools]) ++ [ds_store pyobjc.framework-Quartz];
      format = "pyproject";
      preBuild = ''sed -r 's+/usr/bin/SetFile+${lib.getExe SetFile}+g' -i src/dmgbuild/core.py''; # impure
    };

    mkBadge =
      pkgs.writers.makePythonWriter pythonPackages.python pythonPackages pythonPackages "mkBadge" {
        libraries = [
          (dmgbuild.overrideAttrs (drv: {
            preBuild =
              (drv.preBuild or "")
              + "\n"
              + ''
                sed -r 's/^\s*position = \(0.5, 0.5\)\s*$//g' -i src/dmgbuild/badge.py
                sed -r 's/^def badge_disk_icon\(badge_file, output_file/\0, position/g' -i src/dmgbuild/badge.py
              '';
          }))
        ];
      } ''
        import sys
        import dmgbuild.badge
        if len(sys.argv) != 5:
            print("usage: " + sys.argv[0] + " <source.icns> <target.icns> " +
                  "<posx=0.5> <posy=0.5>")
            sys.exit(1)
        dmgbuild.badge.badge_disk_icon(sys.argv[1], sys.argv[2],
                                       (float(sys.argv[3]), float(sys.argv[4])))
      '';

    badgeIcon = pkgs.runCommand "badge.icns" {} ''
      ${mkBadge} ${svg2icns ./macos-dmg-inset.svg} $out 0.5 0.420
    '';
  }
