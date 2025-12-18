{inputs}: {
  config,
  pkgs,
  ...
}: let
  inherit (pkgs) lib;
  internal = inputs.self.internal.${pkgs.system};
in {
  name = "blockfrost-gateway-devshell";

  imports = [
    "${inputs.devshell}/extra/language/c.nix"
    "${inputs.devshell}/extra/language/rust.nix"
  ];

  devshell.packages =
    [pkgs.unixtools.xxd]
    ++ lib.optionals pkgs.stdenv.isLinux [
      pkgs.pkg-config
    ]
    ++ lib.optionals pkgs.stdenv.isDarwin [
      pkgs.libiconv
    ];

  commands = [
    {package = inputs.self.formatter.${pkgs.system};}
    {package = config.language.rust.packageSet.cargo;}
    {package = pkgs.cargo-nextest;}
    # TODO: add .envrc.local with node env. exports
    {
      name = "cardano-cli";
      package = internal.cardano-cli;
    }
    {package = pkgs.rust-analyzer;}
    {package = pkgs.doctl;}
    {package = internal.hydra-node;}
  ];

  language.c = {
    compiler =
      if pkgs.stdenv.isLinux
      then pkgs.gcc
      else pkgs.clang;
    includes = internal.commonArgs.buildInputs;
  };

  language.rust.packageSet =
    pkgs.rustPackages
    // {
      inherit (internal) rustfmt;
    };

  env =
    (map (network: {
      name = "HYDRA_SCRIPTS_TX_ID_${lib.strings.toUpper network}";
      value = (builtins.fromJSON (builtins.readFile internal.hydraNetworksJson)).${network}.${internal.hydraVersion};
    }) ["mainnet" "preprod" "preview"])
    ++ lib.optionals pkgs.stdenv.isDarwin [
      {
        name = "LIBCLANG_PATH";
        value = internal.commonArgs.LIBCLANG_PATH;
      }
    ]
    ++ lib.optionals pkgs.stdenv.isLinux [
      # Embed `openssl` in `RPATH`:
      {
        name = "RUSTFLAGS";
        eval = ''"-C link-arg=-Wl,-rpath,$(pkg-config --variable=libdir openssl)"'';
      }
    ];

  devshell.motd = ''

    {202}🔨 Welcome to ${config.name}{reset}
    $(menu)

    You can now run ‘{bold}cargo run{reset}’.
  '';
}
