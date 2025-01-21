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
    "${inputs.devshell}/extra/language/rust.nix"
  ];

  devshell.packages = [
      pkgs.unixtools.xxd
      pkgs.openssl
    ] ++ lib.optionals pkgs.stdenv.isLinux [
      pkgs.pkg-config
    ]
    ++ lib.optionals pkgs.stdenv.isDarwin [
      pkgs.libiconv
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
      name = "cardano-address";
      package = internal.cardano-address;
    }
    {package = config.language.rust.packageSet.cargo;}
    {package = pkgs.cargo-nextest;}
    {package = pkgs.rust-analyzer;}
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
      package = internal.tx-build;
    }
    {
      category = "handy";
      name = "testgen-hs";
      package = internal.testgen-hs;
    }
  ];

  language.c.compiler =
    if pkgs.stdenv.isLinux
    then pkgs.gcc
    else pkgs.clang;
  language.c.includes = internal.commonArgs.buildInputs;

  devshell.motd = ''

    {202}🔨 Welcome to ${config.name}{reset}
    $(menu)

    You can now run ‘{bold}cargo run{reset}’.
  '';

  env = [
    {
      name = "TESTGEN_HS_PATH";
      value = lib.getExe internal.testgen-hs;
    }
  ];

  devshell.startup.symlink-configs.text = ''
    ln -sfn ${internal.cardano-node-configs} $PRJ_ROOT/cardano-node-configs
  '';
}
