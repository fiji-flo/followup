{
  description = "Firefox Enterprise event signup service";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      forAllSystems =
        function:
        (nixpkgs.lib.genAttrs [
          "aarch64-darwin"
          "aarch64-linux"
          "x86_64-darwin"
          "x86_64-linux"
        ])
          (
            system:
            function (
              import nixpkgs {
                inherit system;
              }
            )
          );
    in
    {
      packages = forAllSystems (pkgs: {
        followup = pkgs.rustPlatform.buildRustPackage {
          pname = "followup";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl ];

          meta = {
            mainProgram = "followup";
          };
        };

        default = self.packages.${pkgs.stdenv.system}.followup;
      });

      nixosModules.followup =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          cfg = config.services.followup;
        in
        {
          options.services.followup = {
            enable = lib.mkEnableOption "the Firefox Enterprise event signup service";

            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${pkgs.stdenv.system}.followup;
              description = "The followup package to run.";
            };

            bindAddr = lib.mkOption {
              type = lib.types.str;
              default = "0.0.0.0:8080";
              description = "Address and port to bind to.";
            };

            dataDir = lib.mkOption {
              type = lib.types.path;
              default = "/var/lib/followup";
              description = "Directory holding the SQLite database.";
            };

            rpId = lib.mkOption {
              type = lib.types.str;
              description = "WebAuthn RP ID — the bare host (must be a suffix of rpOrigin's host).";
              example = "enterprise.firefox.com";
            };

            rpOrigin = lib.mkOption {
              type = lib.types.str;
              description = "Full origin URL the browser sees.";
              example = "https://enterprise.firefox.com";
            };

            rpName = lib.mkOption {
              type = lib.types.str;
              default = "Firefox Enterprise";
              description = "Human-readable relying-party name.";
            };

            sessionSecure = lib.mkOption {
              type = lib.types.bool;
              default = true;
              description = "Whether to set the `Secure` flag on the session cookie. Disable only for local plain-HTTP testing.";
            };

            environmentFile = lib.mkOption {
              type = with lib.types; nullOr path;
              default = null;
              example = "/run/secrets/followup.env";
              description = ''
                File containing the `EXPORT_TOKEN` and `ADMIN_TOKEN` environment variables
                (the bearer tokens guarding `GET /api/export` and
                `POST /api/admin/phase2/activate` respectively — they should differ), as
                defined in {manpage}`systemd.exec(5)`.
              '';
            };

            logFilter = lib.mkOption {
              type = lib.types.str;
              default = "info";
              description = "RUST_LOG filter.";
            };
          };

          config = lib.mkIf cfg.enable {
            systemd.services.followup = {
              description = "Firefox Enterprise event signup service";
              wantedBy = [ "multi-user.target" ];
              after = [ "network.target" ];

              environment = {
                BIND_ADDR = cfg.bindAddr;
                DATABASE_URL = "sqlite://${cfg.dataDir}/app.db?mode=rwc";
                RP_ID = cfg.rpId;
                RP_ORIGIN = cfg.rpOrigin;
                RP_NAME = cfg.rpName;
                SESSION_SECURE = lib.boolToString cfg.sessionSecure;
                RUST_LOG = cfg.logFilter;
              };

              serviceConfig = {
                ExecStart = lib.getExe cfg.package;
                EnvironmentFile = lib.mkIf (cfg.environmentFile != null) [ cfg.environmentFile ];
                DynamicUser = true;
                StateDirectory = "followup";
                Restart = "on-failure";
              };
            };
          };
        };
    };
}
