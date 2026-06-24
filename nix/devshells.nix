{inputs}: {
  config,
  pkgs,
  ...
}: let
  inherit (pkgs) lib;
  internal = inputs.self.internal.${pkgs.stdenv.hostPlatform.system};
in {
  name = "blockfrost-platform-devshell";

  imports = [
    "${inputs.devshell}/extra/language/c.nix"
    "${inputs.devshell}/extra/language/rust.nix"
  ];

  commands = [
    {package = inputs.self.formatter.${pkgs.stdenv.hostPlatform.system};}
    {
      name = "cardano-node";
      package = internal.cardano-node;
    }
    {
      name = "cardano-cli";
      package = internal.cardano-cli;
    }
    {
      name = "cardano-submit-api";
      package = internal.cardano-submit-api;
    }
    {
      name = "cardano-address";
      package = internal.cardano-address;
    }
    {package = internal.mithril-client;}
    {package = internal.hydra-node;}
    {package = internal.dolos;}
    {package = pkgs.cargo-nextest;}
    {package = pkgs.cargo-tarpaulin;}
    {
      name = "cargo";
      package = internal.rustPackages.cargo;
    }
    {package = internal.rustPackages.rust-analyzer;}
    {package = pkgs.doctl;}
    {
      category = "handy";
      package = internal.runNode "preview";
    }
    {
      category = "handy";
      package = internal.runNode "preprod";
    }
    {
      category = "handy";
      package = internal.runNode "mainnet";
    }
    {
      category = "handy";
      package = internal.runDolos "preview";
    }
    {
      category = "handy";
      package = internal.runDolos "preprod";
    }
    {
      category = "handy";
      package = internal.runDolos "mainnet";
    }
    {
      category = "handy";
      package = internal.tx-build;
    }
    {
      category = "handy";
      name = "testgen-hs";
      package = internal.testgen-hs;
    }
    {
      category = "handy";
      package = internal.run-blockfrost-tests;
    }
    {
      category = "handy";
      package = internal.hydra-test;
    }
  ];

  language.c = {
    compiler =
      if pkgs.stdenv.isLinux
      then pkgs.gcc
      else pkgs.clang;
    includes = internal.commonArgs.buildInputs;
    libraries = internal.commonArgs.buildInputs;
  };

  language.rust = {
    packageSet = internal.rustPackages;
    tools = ["cargo" "rustfmt"]; # The rest is provided below.
    enableDefaultToolchain = true;
  };

  env =
    internal.hydraScriptsEnvVars
    ++ [
      {
        name = "TESTGEN_HS_PATH";
        value = lib.getExe internal.testgen-hs;
      }
    ]
    ++ lib.optionals pkgs.stdenv.isDarwin [
      {
        name = "LIBCLANG_PATH";
        value = internal.commonArgs.LIBCLANG_PATH;
      }
      # `numtide/devshell` sets `LIBRARY_PATH` with a stray `-L` prefix on
      # Darwin, so clang ends up with a bogus `-L-L…/lib` search path and
      # can't find libraries living in the devshell (e.g. `-liconv`).
      {
        name = "LIBRARY_PATH";
        eval = "$DEVSHELL_DIR/lib";
      }
    ]
    ++ lib.optionals pkgs.stdenv.isLinux [
      # Embed `openssl` in `RPATH`:
      {
        name = "RUSTFLAGS";
        eval = ''"-Clink-arg=-fuse-ld=bfd -Clink-arg=-Wl,-rpath,$(pkg-config --variable=libdir openssl libpq | tr ' ' :)"'';
      }
      {
        name = "LD_LIBRARY_PATH";
        eval = lib.mkForce "";
      }
    ];

  devshell = {
    packages =
      [
        pkgs.unixtools.xxd
        internal.rustPackages.clippy
        pkgs.hadolint
        pkgs.websocat
      ]
      ++ lib.optionals pkgs.stdenv.isLinux [
        pkgs.pkg-config
      ]
      ++ lib.optionals pkgs.stdenv.isDarwin [
        pkgs.libiconv
      ];

    motd = ''

      {202}🔨 Welcome to ${config.name}{reset}
      $(menu)

      You can now run ‘{bold}cargo run{reset}’.
    '';

    startup.symlink-configs.text = ''
      for old_link in cardano-node-configs dolos-configs ; do
        if [[ -L "$PRJ_ROOT/$old_link" ]] ; then rm -- "$PRJ_ROOT/$old_link" ; fi
      done

      ln -sfn ${internal.generated-dir} "$PRJ_ROOT/generated"
    '';

    startup.install-git-hooks.text = let
      gitHooks = pkgs.runCommand "blockfrost-platform-git-hooks" {} ''
        mkdir -p "$out"
        ln -s ${
          pkgs.writeShellScript "pre-commit" ''
            set -euo pipefail

            if [ -n "''${SKIP_TREEFMT+x}" ]; then
              exit 0
            fi

            if "${lib.getExe inputs.self.formatter.${pkgs.stdenv.hostPlatform.system}}" --fail-on-change; then
              exit 0
            else
              status=$?
              echo >&2 'error: treefmt detected (and corrected) unformatted code'
              echo >&2 'hint: set SKIP_TREEFMT=1 to skip treefmt in pre-commit'
              exit "$status"
            fi
          ''
        } "$out/pre-commit"
      '';
    in ''
      if [[ -e "$PRJ_ROOT/.git" ]] ; then
        # Only manage `core.hooksPath` when it's unset or still points at a
        # (possibly older) version of our own hooks; never clobber a
        # contributor's own setup or hooks installed by other tooling.
        installed=
        current="$(git -C "$PRJ_ROOT" config --get core.hooksPath 2>/dev/null || true)"
        if [[ "$current" == "${gitHooks}" ]] ; then
          installed=1
        elif [[ -z "$current" || "$current" == /nix/store/*-blockfrost-platform-git-hooks ]] ; then
          if git -C "$PRJ_ROOT" config --local core.hooksPath ${gitHooks} ; then
            installed=1
          else
            echo >&2 "note: couldn't set git core.hooksPath (is .git read-only?)"
          fi
        else
          echo >&2 "note: leaving your existing git core.hooksPath ($current) untouched"
        fi
        if [[ -z "$installed" ]] ; then
          echo >&2 "hint: the blockfrost-platform pre-commit (treefmt) hook was not installed"
        fi
        unset installed current
      fi
    '';
  };
}
