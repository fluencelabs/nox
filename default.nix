# default.nix
with import <nixpkgs> { };
stdenv.mkDerivation {
  name = "dev-environment"; # Probably put a more meaningful name here
  buildInputs = [ pkg-config hwloc iconv cmake ] ++ lib.optionals pkgs.stdenv.isDarwin [
    darwin.apple_sdk.frameworks.SystemConfiguration
  ];
}
