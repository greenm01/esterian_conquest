{
  description = "Nostrian Conquest development shell";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = f:
        nixpkgs.lib.genAttrs systems (system:
          f {
            pkgs = import nixpkgs { inherit system; };
          });
    in
    {
      devShells = forAllSystems ({ pkgs }: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            pkg-config
            python3
            rust-analyzer
            rustc
            rustfmt
            sccache
            stdenv.cc
            wayland
            wayland-protocols
            libxkbcommon
            vulkan-loader
            mesa
            libGL
            fontconfig
            freetype
            libx11
            libxcursor
            libxi
            libxinerama
            libxrandr
            libxcb
          ];

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
            pkgs.wayland
            pkgs.libxkbcommon
            pkgs.vulkan-loader
            pkgs.mesa
            pkgs.libGL
            pkgs.fontconfig
            pkgs.freetype
            pkgs.libx11
            pkgs.libxcursor
            pkgs.libxi
            pkgs.libxinerama
            pkgs.libxrandr
            pkgs.libxcb
          ];

          RUSTC_WRAPPER = "sccache";
        };
      });
    };
}
