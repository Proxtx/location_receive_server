{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = {self, nixpkgs, flake-utils, crane}:
    flake-utils.lib.eachDefaultSystem(system: 
      let 
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.mkLib pkgs;
      in {
        packages.default = craneLib.buildPackage {
          src = craneLib.cleanCargoSource ./.;
        };
      }) // {
      nixosModules.default = {config, lib, pkgs, ...} : {
        options.services.location_receive_server = {
          enable = pkgs.lib.mkEnableOption "Location Receive Server";
          config = lib.mkOption {
            type = lib.types.attrs;
            default = {};
            description = "Configuration for the Location Receive Server.";
          };
          data_dir = lib.mkOption {
            type = lib.types.path;
            default = "/var/lib/location_receive_server/";
            description = "Sets the current working directory";
          };
          package = lib.mkOption {
            type = lib.types.package;
            default = self.packages.${pkgs.system}.default;
            description = "Location Receive Server";
          };
        };

        config = lib.mkIf config.services.location_receive_server.enable {
          
          users.groups = {
            location_receive_server = {};
          };

          users.users = {
            location_receive_server = {
              group = "location_receive_server";
              isSystemUser = true;
            };
          };

          system.activationScripts.copyConfigLocationReceiveServer = ''
            cp ${(pkgs.formats.toml {}).generate "location_receive_server_config" config.services.location_receive_server.config} ${config.services.location_receive_server.data_dir}/config.toml
            mkdir -p ${config.services.location_receive_server.data_dir}/locations
            mkdir -p ${config.services.location_receive_server.data_dir}/data
            chown -R location_receive_server:location_receive_server ${config.services.location_receive_server.data_dir}
          '';

          systemd.services.location_receive_server = {
            wantedBy = ["multi-user.target"];
            serviceConfig = {
              ExecStart = "${config.services.location_receive_server.package}/bin/location_receive_server";
              Restart = "always";
              User = "location_receive_server";
              WorkingDirectory = "${config.services.location_receive_server.data_dir}";
              Group = "location_receive_server";
            };
          };

          systemd.tmpfiles.settings = {
            "locationReceiverServerStorage".${config.services.location_receive_server.data_dir}.d = {
              user = "location_receive_server";
              group = "location_receive_server";
              mode = "0770";
            };
          };
        };
      };
    };
}