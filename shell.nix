{ pkgs ? import
    (builtins.fetchTarball {
      url = "https://github.com/NixOS/nixpkgs/archive/ec25c90d35d24e36c0af3b3d58a09542367ebbee.tar.gz"; # nixpkgs-unstable
      sha256 = "0g7r3v3n8w5saw5zgr6lz8ip2gfkq2mwyp6ki7lr5l0jllyb1v4g";
    })
    { }
}:
with pkgs;
pkgs.mkShell {
  buildInputs = [ pkgs.cargo pkgs.rustc ] ++ lib.optional stdenv.isDarwin [ pkgs.darwin.apple_sdk.frameworks.Security pkgs.darwin.apple_sdk.frameworks.SystemConfiguration ];
}
