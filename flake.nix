{
  description = "Pinned Rust/Emscripten Cloudflare Worker toolchain";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    worker-toolchain.url = "github:NixOS/nixpkgs/baa82799b627d760265eae9965b54c180462de23";
    rust-overlay = {
      url = "github:oxalica/rust-overlay/5424e1e6dda30b0626935d2a03a6526db4858d60";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { nixpkgs, worker-toolchain, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        toolPkgs = import worker-toolchain { inherit system; };
        rustToolchain = pkgs.rust-bin.stable."1.97.0".minimal.override {
          targets = [ "wasm32-unknown-emscripten" ];
        };
        # SDK 6.0.10 supplies LLVM 24; Nixpkgs LLVM 22 lacks its WASM builtins.
        emscriptenSdk = pkgs.stdenvNoCC.mkDerivation {
          pname = "emscripten-sdk-llvm";
          version = "24.0.0-sdk-6.0.10";
          src = pkgs.fetchurl {
            name = "emscripten-sdk-6.0.10.tar.xz";
            url = "https://storage.googleapis.com/webassembly/emscripten-releases-builds/linux/666337b525e673e769121856d175f6f52b8ead64/wasm-binaries.tar.xz";
            hash = "sha256-pR3Sgpvfcl0miXRyjgMlM7EG6gHLSCpiKI7BJienq74=";
          };
          nativeBuildInputs = [ pkgs.autoPatchelfHook ];
          buildInputs = [ pkgs.stdenv.cc.cc.lib pkgs.zlib pkgs.ncurses pkgs.libxml2 pkgs.libffi ];
          dontConfigure = true;
          dontBuild = true;
          meta.platforms = [ "x86_64-linux" ];
          installPhase = ''
            runHook preInstall
            mkdir -p "$out"
            cp -a . "$out/"
            runHook postInstall
          '';
        };
        # 6.0.10 fixes wasm-bindgen dependencies for source-phase imports.
        emscripten = (toolPkgs.emscripten.override {
          nodejs = pkgs.nodejs_24;
          buildNpmPackage = pkgs.buildNpmPackage;
          python3 = pkgs.python3;
          jre = pkgs.jre;
          closurecompiler = pkgs.closurecompiler;
          llvmPackages = {
            clang-unwrapped = emscriptenSdk;
            lld = emscriptenSdk;
            llvm = emscriptenSdk;
          };
          binaryen = emscriptenSdk;
        }).overrideAttrs (old: {
          version = "6.0.10";
          src = toolPkgs.fetchurl {
            name = "emscripten-6.0.10.tar.gz";
            url = "https://codeload.github.com/emscripten-core/emscripten/tar.gz/refs/tags/6.0.10";
            hash = "sha256-GC9LiqK2ScQ02hmYhsY38ysBcln5eQdVTnwxOFz0AbY=";
          };
          # The Nixpkgs compatibility patch is only needed before LLVM 23.
          patches = builtins.filter
            (patch: !(pkgs.lib.hasSuffix "0002-libunwind-restore-Unwind_CallPersonality.patch" (toString patch)))
            old.patches;
        });
        bindgenSource = pkgs.fetchCrate {
          pname = "wasm-bindgen-cli";
          version = "0.2.129";
          hash = "sha256-pcecKQd7E8Opw6bkFoE569epUi7gh5qpQF1e5PJY6V8=";
        };
        wasm-bindgen-cli = pkgs.callPackage
          "${worker-toolchain}/pkgs/build-support/wasm-bindgen-cli" { } {
            version = "0.2.129";
            src = bindgenSource;
            cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
              src = bindgenSource;
              hash = "sha256-vmUrWVU7kPJJxO5qIVeAkwQyWDELO1Z4Z5gitz2kco8=";
            };
          };
      in
      {
        packages = { inherit emscripten wasm-bindgen-cli; };

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            bash
            coreutils
            curl
            emscripten
            git
            iproute2
            jq
            nodejs_24 # Node.js
            pnpm # Package manager
            ripgrep
            rustToolchain
            shellcheck
            sops
            util-linux
            yq
            just # Task runner
            wasm-bindgen-cli # WASM bindgen CLI
            toolPkgs.wrangler # Includes the runtime required by the Emscripten loader
          ];

          shell = "${pkgs.bash}/bin/bash";

          shellHook = ''
            export EM_CACHE="$PWD/.cache/emscripten"
            if [ ! -d node_modules ] && [ -f pnpm-lock.yaml ]; then
              pnpm install --frozen-lockfile
            fi
          '';
        };
      }
    );
}
