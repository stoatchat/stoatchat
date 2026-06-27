{ pkgs ? import (fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/77ef7a29d276c6d8303aece3444d61118ef71ac2.tar.gz";
    sha256 = "0pm4l48jq8plzrrrisimahxqlcpx7qqq9c99hylmf7p3zlc3phsy";
  }) {},
}:

let
  nix-ld-libs = pkgs.buildEnv {
    name = "nix-ld-libs";
    paths = with pkgs; [
      stdenv.cc.cc.lib
      zlib
      openssl.out
    ];
    pathsToLink = [ "/lib" ];
  };

in pkgs.mkShell {
  packages = with pkgs; [
    mise
    cargo-binstall
    (writeShellScriptBin "fish" ''
      exec ${pkgs.fish}/bin/fish -C 'mise activate fish | source' "$@"
    '')
  ];

  buildInputs = with pkgs; [
    pkg-config
    openssl.dev
  ];

  shellHook = ''
    export TMPDIR=/tmp
    export NIX_LD="${pkgs.stdenv.cc.libc}/lib/ld-linux-x86-64.so.2"
    export NIX_LD_LIBRARY_PATH="${nix-ld-libs}/lib"
    export LD_LIBRARY_PATH="${nix-ld-libs}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

    export MISE_NODE_COMPILE=false
    eval "$(mise activate bash)"
  '';
}