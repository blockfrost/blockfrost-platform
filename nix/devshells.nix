{inputs}: {
  config,
  pkgs,
  ...
}: let
  inherit (pkgs) lib;
  internal = inputs.self.internal.${pkgs.system};
in {
  name = "blockfrost-platform-devshell";

  imports = [
    "${inputs.devshell}/extra/language/c.nix"
  ];

  commands = [
    {package = inputs.self.formatter.${pkgs.system};}
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
    {package = internal.dolos;}
    {package = pkgs.cargo-nextest;}
    {package = pkgs.cargo-tarpaulin;}
    {
      name = "cargo";
      package = internal.rustPackages.cargo;
    }
    {package = internal.rustPackages.rust-analyzer;}
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
      package = internal.tx-build;
    }
    {
      category = "handy";
      name = "testgen-hs";
      package = internal.testgen-hs;
    }
  ];

  language.c = {
    compiler =
      if pkgs.stdenv.isLinux
      then pkgs.gcc
      else pkgs.clang;
    includes = internal.commonArgs.buildInputs;
  };

  env =
    [
      {
        name = "TESTGEN_HS_PATH";
        value = lib.getExe internal.testgen-hs;
      }
      {
        name = "RUST_SRC_PATH";
        value = "${internal.rustPackages.rust-src}/lib/rustlib/src/rust/library";
      }
    ]
    ++ lib.optionals pkgs.stdenv.isDarwin [
      {
        name = "LIBCLANG_PATH";
        value = internal.commonArgs.LIBCLANG_PATH;
      }
      {
        name = "LIBRARY_PATH";
        value = "${pkgs.libiconv}/lib";
      }
    ];

  devshell = {
    packages =
      [
        pkgs.unixtools.xxd
        internal.rustPackages.clippy
      ]
      ++ lib.optionals pkgs.stdenv.isLinux [
        pkgs.pkg-config
        pkgs.wget
        pkgs.curl
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
      ln -sfn ${internal.cardano-node-configs} $PRJ_ROOT/cardano-node-configs
    '';
  };
}
