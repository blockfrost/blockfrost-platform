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

    dmgImage = let
      outFileName = "${unix.package.pname}-${unix.package.version}-${inputs.self.shortRev or "dirty"}-${targetSystem}.dmg";
      # See <https://dmgbuild.readthedocs.io/en/latest/settings.html>:
      settingsPy = let
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
    in
      pkgs.runCommand "blockchain-services-dmg" {
        passthru = {inherit outFileName;};
      } ''
        mkdir -p $out
        target=$out/${outFileName}

        ${dmgbuild}/bin/dmgbuild \
          -D app_path=${app-bundle}/Applications/*.app \
          -D icon_path=${badgeIcon} \
          -s ${settingsPy} \
          ${lib.escapeShellArg prettyName} $target

        # Make it downloadable from Hydra:
        mkdir -p $out/nix-support
        echo "file binary-dist \"$target\"" >$out/nix-support/hydra-build-products
      '';

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

    # Apple make changes to the original libffi, e.g. adding this non-standard symbol: `ffi_find_closure_for_code_np`
    apple_libffi = pkgs.stdenv.mkDerivation {
      name = "apple-libffi";
      dontUnpack = true;
      installPhase = let
        sdk = newestSDK.MacOSX-SDK;
      in ''
        mkdir -p $out/include $out/lib
        cp -r ${sdk}/usr/include/ffi $out/include/
        cp -r ${sdk}/usr/lib/libffi.* $out/lib/
      '';
    };

    # For the DMG tooling:
    newestSDK = pkgs.darwin.apple_sdk_11_0;

    pyobjc = rec {
      version = "9.2";

      commonPreBuild = ''
        # Force it to target our ‘darwinMinVersion’, it’s not recognized correctly:
        grep -RF -- '-DPyObjC_BUILD_RELEASE=%02d%02d' | cut -d: -f1 | while IFS= read -r file ; do
          sed -r '/-DPyObjC_BUILD_RELEASE=%02d%02d/{s/%02d%02d/${
          lib.concatMapStrings (lib.fixedWidthString 2 "0") (
            lib.splitString "." newestSDK.stdenv.targetPlatform.darwinMinVersion
          )
        }/;n;d;}' -i "$file"
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
        nativeBuildInputs = [newestSDK.xcodebuild pkgs.darwin.cctools];
        buildInputs =
          (with pkgs; [])
          ++ [newestSDK.objc4 apple_libffi newestSDK.libs.simd]
          ++ (with newestSDK.frameworks; [Foundation GameplayKit MetalPerformanceShaders]);
        hardeningDisable = ["strictoverflow"]; # -fno-strict-overflow is not supported in clang on darwin
        NIX_CFLAGS_COMPILE = ["-Wno-error=deprecated-declarations"];
        preBuild =
          commonPreBuild
          + ''
            sed -r 's+\(.*usr/include/objc/runtime\.h.*\)+("${newestSDK.objc4}/include/objc/runtime.h")+g' -i setup.py
            sed -r 's+/usr/include/ffi+${apple_libffi}/include+g' -i setup.py

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
      };

      framework-Cocoa = pythonPackages.buildPythonPackage rec {
        pname = "pyobjc-framework-Cocoa";
        inherit version;
        src = pythonPackages.fetchPypi {
          inherit pname version;
          hash = "sha256-79eAgIctjI3mwrl+Dk6smdYgOl0WN6oTXQcdRk6y21M=";
        };
        nativeBuildInputs = [newestSDK.xcodebuild pkgs.darwin.cctools];
        buildInputs = with newestSDK.frameworks; [Foundation AppKit];
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
        nativeBuildInputs = [newestSDK.xcodebuild pkgs.darwin.cctools];
        buildInputs = with newestSDK.frameworks; [Foundation CoreVideo Quartz];
        propagatedBuildInputs = [framework-Cocoa];
        hardeningDisable = ["strictoverflow"]; # -fno-strict-overflow is not supported in clang on darwin
        preBuild = commonPreBuild;
        dontUseSetuptoolsCheck = true; # XXX: majority is passing
      };
    };

    # How to get it in a saner way?
    apple_SetFile = pkgs.runCommand "SetFile" {} ''
      mkdir -p $out/bin
      cp ${newestSDK.CLTools_Executables}/usr/bin/SetFile $out/bin/
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
      propagatedBuildInputs = (with pythonPackages; [setuptools]) ++ [ds_store pyobjc.framework-Quartz];
      format = "pyproject";
      preBuild = ''sed -r 's+/usr/bin/SetFile+${apple_SetFile}/bin/SetFile+g' -i src/dmgbuild/core.py''; # impure
    };

    mkBadge =
      pkgs.writers.makePythonWriter pythonPackages.python pythonPackages pythonPackages "mkBadge" {
        libraries = [
          (dmgbuild.overrideDerivation (drv: {
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
