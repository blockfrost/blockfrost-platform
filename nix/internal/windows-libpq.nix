# Cross-compile just libpq (PostgreSQL client library) for Windows using MinGW.
#
# We can't use `pkgsCross.mingwW64.postgresql` because the full PostgreSQL
# package is broken for Windows in Nixpkgs. Instead, we build only the client
# library (libpq) from source using Meson cross-compilation.
#
# libpq depends on internal PostgreSQL libraries (pgcommon_shlib, pgport_shlib),
# so we build those too and merge everything into a single static archive.
{
  pkgs,
  pkgsCross,
}: let
  inherit (pkgs) lib;

  postgresqlSrc = pkgs.postgresql.src;

  crossPrefix = pkgsCross.stdenv.cc.targetPrefix;
  crossAr = "${pkgsCross.stdenv.cc}/bin/${crossPrefix}ar";

  crossFile = pkgs.writeText "cross-file.txt" ''
    [binaries]
    c = '${crossPrefix}cc'
    cpp = '${crossPrefix}c++'
    ar = '${crossPrefix}ar'
    strip = '${crossPrefix}strip'
    windres = '${crossPrefix}windres'
    pkgconfig = 'pkg-config'

    [properties]
    sys_root = '${pkgsCross.stdenv.cc}/${crossPrefix}'

    [host_machine]
    system = 'windows'
    cpu_family = 'x86_64'
    cpu = 'x86_64'
    endian = 'little'
  '';
in
  pkgs.stdenv.mkDerivation {
    pname = "libpq-windows";
    inherit (pkgs.postgresql) version;

    src = postgresqlSrc;

    nativeBuildInputs = [
      pkgs.meson
      pkgs.ninja
      pkgs.pkg-config
      pkgs.bison
      pkgs.flex
      pkgs.perl
      pkgsCross.stdenv.cc
    ];

    buildInputs = [
      pkgsCross.windows.pthreads
    ];

    # We use Meson's cross-compilation support. We only build the `libpq`
    # sub-target so we don't need to deal with the full PostgreSQL build.
    configurePhase = ''
      runHook preConfigure
      meson setup build \
        --cross-file ${crossFile} \
        -Dprefix=$out \
        -Ddefault_library=both \
        -Dnls=disabled \
        -Dreadline=disabled \
        -Dzlib=disabled \
        -Dssl=none \
        -Dgssapi=disabled \
        -Dldap=disabled \
        -Dpam=disabled \
        -Dselinux=disabled \
        -Dsystemd=disabled \
        -Duuid=none \
        -Dicu=disabled \
        -Dlz4=disabled \
        -Dzstd=disabled \
        -Dplperl=disabled \
        -Dplpython=disabled \
        -Dpltcl=disabled \
        -Dllvm=disabled \
        -Dlibxml=disabled \
        -Dlibxslt=disabled
      runHook postConfigure
    '';

    buildPhase = ''
      runHook preBuild
      # Build libpq and its internal dependencies (pgcommon, pgport):
      ninja -C build \
        src/interfaces/libpq/libpq.a \
        src/common/libpgcommon.a \
        src/common/libpgcommon_shlib.a \
        src/port/libpgport.a \
        src/port/libpgport_shlib.a
      runHook postBuild
    '';

    installPhase = ''
      runHook preInstall
      mkdir -p $out/lib $out/include

      # Merge all static archives into a single libpq.a so that the linker
      # can find all symbols (pgcommon, pgport) that libpq needs:
      tmpdir=$(mktemp -d)
      for archive in \
        build/src/interfaces/libpq/libpq.a \
        build/src/common/libpgcommon.a \
        build/src/common/libpgcommon_shlib.a \
        build/src/port/libpgport.a \
        build/src/port/libpgport_shlib.a \
      ; do
        if [ -f "$archive" ]; then
          (cd "$tmpdir" && ${crossAr} x "$OLDPWD/$archive")
        fi
      done
      ${crossAr} rcs $out/lib/libpq.a "$tmpdir"/*.obj "$tmpdir"/*.o 2>/dev/null \
        || ${crossAr} rcs $out/lib/libpq.a "$tmpdir"/*

      # Install headers:
      cp src/include/postgres_ext.h $out/include/
      cp src/include/pg_config_ext.h $out/include/ 2>/dev/null || true
      cp src/interfaces/libpq/libpq-fe.h $out/include/
      cp src/interfaces/libpq/libpq-events.h $out/include/
      cp build/src/include/pg_config.h $out/include/ 2>/dev/null || true
      cp build/src/include/pg_config_ext.h $out/include/ 2>/dev/null || true

      runHook postInstall
    '';

    meta = {
      description = "PostgreSQL client library (libpq) cross-compiled for Windows";
      license = lib.licenses.postgresql;
    };
  }
