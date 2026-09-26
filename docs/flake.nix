{
  description = "Development environment for stoatchat docs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
    in {
      devShells = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [
              nodejs
              pnpm
            ];

            shellHook = ''
              export SUPPRESS_NO_CONFIG_WARNING=true
              alias gen_events_doc="npx @asyncapi/cli generate fromTemplate docs/developers/events/asyncapi.yml @asyncapi/markdown-template --output docs/developers/events/ -p outFilename=protocols.md"
            '';
          };
        });
    };
}
