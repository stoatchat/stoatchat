{
  description = "Development environment for stoatchat";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forEachSystem = nixpkgs.lib.genAttrs supportedSystems;
    in
    {
      devShells = forEachSystem (system:
        let
          pkgs = import nixpkgs { inherit system; };

          nix-ld-libs = pkgs.buildEnv {
            name = "nix-ld-libs";
            paths = with pkgs; [
              stdenv.cc.cc.lib
              zlib
              openssl.out
              dav1d
            ];
            pathsToLink = [ "/lib" ];
          };
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              mise
              cargo-binstall
              pkg-config
              openssl
              openssl.dev
              meson
              ninja
              mold
              nasm
            ];

            shellHook = ''
              # Crucial for rust openssl-sys compilation via cargo
              export PKG_CONFIG_PATH="\({pkgs.openssl.dev}/lib/pkgconfig:\){pkgs.dav1d}/lib/pkgconfig\({PKG_CONFIG_PATH:+:\)PKG_CONFIG_PATH}"
              export OPENSSL_DIR="${pkgs.openssl.dev}"
              export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"

              export MISE_NODE_COMPILE=false
              if command -v mise &> /dev/null; then
                eval "$(mise activate bash)"
              fi
            '';
          };
        }
      );
    };
}
