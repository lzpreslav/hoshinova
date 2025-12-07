{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    cargo
    nodejs
    rustc
    typescript
    yarn
    ytarchive
  ];
}
