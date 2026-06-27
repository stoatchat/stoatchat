{
  pkgs ? import <nixpkgs> { },
}:

with pkgs;
pkgs.mkShell {
  buildInputs = [
    nodejs
    nodejs.pkgs.pnpm
  ];
}
