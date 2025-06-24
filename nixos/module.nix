{
  pkgs,
  lib,
  config,
  ...
}:

with lib;

let
  cfg = config.services.satisfactory-serve-map;

  tomlFormat = pkgs.formats.toml { };

  configFile = tomlFormat.generate "config.toml" {
    base_url = cfg.base_url;
    save_dir = cfg.save_dir;
    port = cfg.port;
  };
in
{
  options.services.satisfactory-serve-map = {
    enable = mkEnableOption (lib.mdDoc "satisfactory-serve-map service");

    port = mkOption {
      type = types.port;
      default = 7778;
      description = lib.mdDoc "Port to listen on.";
    };

    base_url = mkOption {
      type = types.str;
      description = lib.mdDoc "Base URL for constructing map links.";
      example = "https://sf.example.com";
    };

    save_dir = mkOption {
      type = types.path;
      description = lib.mdDoc ''
        Directory containing save files.
        The user running the service needs read access to this directory.
      '';
      example = "/var/lib/satisfactory/saves";
    };

    user = mkOption {
      type = types.str;
      default = "satisfactory-serve-map";
      description = lib.mdDoc "User to run the service as.";
    };

    group = mkOption {
      type = types.str;
      default = "satisfactory-serve-map";
      description = lib.mdDoc "Group to run the service as.";
    };
  };

  config = mkIf cfg.enable {
    users.users."${cfg.user}" = {
      isSystemUser = true;
      group = cfg.group;
    };

    users.groups."${cfg.group}" = { };

    systemd.services.satisfactory-serve-map = {
      description = "Satisfactory Serve Map Service";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        User = cfg.user;
        Group = cfg.group;
        ExecStart = "${pkgs.satisfactory-serve-map}/bin/satisfactory_serve_map";
        WorkingDirectory = toString (
          pkgs.linkFarm "satisfactory-serve-map-config" [
            {
              name = "config.toml";
              path = configFile;
            }
          ]
        );
        Restart = "on-failure";
        RestartSec = "5s";
      };
    };
  };
}
