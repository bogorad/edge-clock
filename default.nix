# nix-shell uses the same pinned dev shell as nix develop and direnv.
(builtins.getFlake (toString ./.)).devShells.${builtins.currentSystem}.default
