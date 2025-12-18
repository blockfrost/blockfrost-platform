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
    [
      pkgs.unixtools.xxd
      internal.rustPackages.clippy
    ]
    ++ lib.optionals pkgs.stdenv.isLinux [
      pkgs.pkg-config
    ]
    ++ lib.optionals pkgs.stdenv.isDarwin [
      pkgs.libiconv
    ];

  commands = [
    {package = inputs.self.formatter.${pkgs.system};}
    {
      name = "cargo";
      package = internal.rustPackages.cargo;
    }
    {package = pkgs.cargo-nextest;}
    # TODO: add .envrc.local with node env. exports
    {
      name = "cardano-cli";
      package = internal.cardano-cli;
    }
    {package = internal.rustPackages.rust-analyzer;}
    {package = pkgs.doctl;}
    {package = internal.hydra-node;}
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
    ++ lib.optionals pkgs.stdenv.isDarwin [
      {
        name = "LIBCLANG_PATH";
        value = internal.commonArgs.LIBCLANG_PATH;
      }
    ]
    ++ lib.optionals pkgs.stdenv.isLinux [
      # Embed runtime libs in `RPATH`:
      {
        name = "RUSTFLAGS";
        eval = ''"-Clink-arg=-fuse-ld=bfd -Clink-arg=-Wl,-rpath,$(pkg-config --variable=libdir openssl libpq | tr ' ' :)"'';
      }
      {
        name = "LD_LIBRARY_PATH";
        eval = lib.mkForce "";
      }
    ];

  devshell.motd = ''

    {202}🔨 Welcome to ${config.name}{reset}
    $(menu)

    You can now run ‘{bold}cargo run{reset}’.
  '';
}
