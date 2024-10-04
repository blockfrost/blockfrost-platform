{inputs}: {
  config,
  pkgs,
  ...
}: let
  inherit (pkgs) lib;
in {
  name = "blockfrost-platform-devshell";

  imports = [
    "${inputs.devshell}/extra/language/c.nix"
    "${inputs.devshell}/extra/language/rust.nix"
  ];

  devshell.packages =
    lib.optionals pkgs.stdenv.isLinux [
      pkgs.pkg-config
    ]
    ++ lib.optionals pkgs.stdenv.isDarwin [
      pkgs.libiconv
    ];

  commands = [
    {package = inputs.self.formatter.${pkgs.system};}
    {
      name = "cardano-node";
      package = inputs.self.internal.${pkgs.system}.cardano-node;
    }
    {package = config.language.rust.packageSet.cargo;}
  ];

  language.c.compiler =
    if pkgs.stdenv.isLinux
    then pkgs.gcc
    else pkgs.clang;
  language.c.includes = inputs.self.internal.${pkgs.system}.commonArgs.buildInputs;

  devshell.motd = ''

    {202}🔨 Welcome to ${config.name}{reset}
    $(menu)

    You can now run ‘{bold}cargo run{reset}’.
  '';
}
