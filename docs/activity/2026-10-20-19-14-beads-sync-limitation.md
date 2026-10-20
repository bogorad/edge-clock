# Beads remote sync limitation

Migration: `edge-clock-3oh`. Follow-up: `edge-clock-zu6`.

The shared `edge_clock` tracker at `doltsrv.lan:3307` accepts issue reads and
writes. Its remote sync and push failed because the server could not find
`git` on its command path. The Dolt `origin` remote already exists.

No `bd-init`, `bd init`, tracker reset, or remote repair was run. Inspection of
the routine `bd-init` source showed that it repairs local configuration and
hooks, removes legacy exports, and attaches to the shared database. It does
not repair the server command path. The installed script is a Nix store link;
its source parity was not inspected under the rule against reading `/nix/`.

Filed the exact sync failure for separate service investigation. Source Git
publication and Cloudflare deployment remain separate from this tracker
transport failure.
