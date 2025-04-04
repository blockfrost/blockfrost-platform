{
  config,
  lib,
  pkgs,
  ...
}: let
  inherit (lib) types;
  cfg = config.services.blockfrost-platform;
in {
  options = {
    services.blockfrost-platform = {
      enable = lib.mkEnableOption "enable the blockfrost-platform service";
      args = lib.mkOption {
        description = "Command-line arguments to launch the blockfrost-platform.";
        type = types.listOf types.str;
        default =
          (
            if cfg.secretConfig != null
            then ["--config" cfg.secretConfig]
            else ["--solitary"]
          )
          ++ ["--server-address" cfg.serverAddr]
          ++ ["--server-port" (toString cfg.port)]
          ++ ["--network" cfg.network]
          ++ ["--log-level" cfg.logLevel]
          ++ ["--node-socket-path" cfg.nodeSocket]
          ++ ["--mode" cfg.mode];
      };
      secretConfig = lib.mkOption {
        type = types.nullOr (types.either types.str types.path);
        description = ''
          Path to a config file with secrets of the following form:

          ```
          reward_address = "addr1…"
          secret = "00000000000000000000000000000000"
          ```

          If it’s not defined, `--solitary` mode will be used.
        '';
        default = null;
      };
      nodeSocket = lib.mkOption {
        type = types.nullOr (types.either types.str types.path);
        description = "Path to the cardano-node socket.";
      };
      port = lib.mkOption {
        type = types.int;
        default = 3000;
        description = "The port number.";
      };
      serverAddr = lib.mkOption {
        type = types.str;
        default = "0.0.0.0";
        description = "The host address to bind to.";
      };
      logLevel = lib.mkOption {
        type = types.enum ["debug" "info" "warn" "error" "trace"];
        default = "info";
      };
      mode = lib.mkOption {
        type = types.enum ["compact" "light" "full"];
        default = "compact";
        description = "This doesn’t mean anything yet.";
      };
      network = lib.mkOption {
        type = types.enum ["mainnet" "preprod" "preview"];
        default = "mainnet";
      };
      package = lib.mkOption {
        type = types.package;
        default = pkgs.blockfrost-platform;
      };
    };
  };
  config = lib.mkIf cfg.enable {
    systemd.services.blockfrost-platform = {
      wantedBy = ["multi-user.target"];
      serviceConfig = {
        Type = "simple";
        Restart = "always";
        RestartSec = 5;
        DynamicUser = true;
        ExecStart = lib.escapeShellArgs ([(lib.getExe cfg.package)] ++ cfg.args);
      };
      environment = {
        # For `dirs::config_dir()`:
        XDG_CONFIG_HOME = pkgs.emptyDirectory;
      };
    };
  };
}
