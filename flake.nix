{
  description = "Development shell with Node.js 24 and pnpm";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            lld # WASM linker
            nodejs_24 # Node.js
            pnpm # Package manager
            rustc # Rust compiler
            wasm-pack # WASM packager
            wrangler # Cloudflare worker
            zsh # Shell
          ];

          shell = "${pkgs.zsh}/bin/zsh";

          shellHook = ''
            if [ -f package.json ]; then
              pnpm install
            else
              cat > package.json <<'EOF'
            {
              "name": "devshell-project",
              "version": "0.0.0",
              "private": true
            }
            EOF
              pnpm install
            fi
          '';
        };
      }
    );
}
