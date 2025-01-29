{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {self, nixpkgs, flake-utils}:
    flake-utils.lib.eachDefaultSystem(system: {
        packages.default = nixpkgs.legacyPackages.${system}.rustPlatform.buildRustPackage {
          pname = "location_receive_server";
          version = "0.1.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };
  
          nativeBuildInputs = [];
          buildInputs = [];
        };
    }) // {
      nixosModules.default = {config, lib, pkgs, ...} : {
        options.services.location_receive_server = {
          enable = pkgs.lib.mkEnableOption "Location Receive Server";
          config = lib.mkOption {
            type = lib.types.string;
            default = "";
            description = "Configuration for the Location Receive Server";
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

          #system.activationScripts.copyConfigLocationReceiveServer = ''
          #  cat <<EOF > ${config.services.location_receive_server.data_dir}/config.toml
          #  ${config.services.location_receive_server.config}
          #  EOF
          #'';

          system.activationScripts.copyConfigLocationReceiveServer = ''
            echo ${config.services.location_receive_server.config} >> ${config.services.location_receive_server.data_dir}/config.toml
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