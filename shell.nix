let
  sources = import ./nix/sources.nix;
  rust = import ./nix/rust.nix { inherit sources; };
  pkgs = import sources.nixpkgs { };
  frameworks = pkgs.darwin.apple_sdk.frameworks;
in
pkgs.mkShell {
  buildInputs = [
    rust
    pkgs.sqlite
  ];

  propagatedBuildInputs = with pkgs; [
    frameworks.Security
  ];

   NIX_LDFLAGS = "-F${frameworks.Security}/Library/Frameworks -framework Security";
}
