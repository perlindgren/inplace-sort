{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            config.allowUnfree = true;
          };
        in
        {
          default = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              gnumake
              drawio
              typst
              fontconfig
              gyre-fonts
            ];

            shellHook = ''
              export FONTCONFIG_FILE=${
                pkgs.makeFontsConf {
                  fontDirectories = [
                    "${pkgs.gyre-fonts}/share/fonts"
                  ];
                }
              }
            '';
          };
        }
      );
    };
}
