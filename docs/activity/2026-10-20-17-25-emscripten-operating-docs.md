# Migration operating documentation

Updated the README for the Emscripten build, generated loader, live HTTP gate,
and separate Git/Worker release boundaries. Added one root knowledge map for
the app's shared lifecycle. Existing documentation paths remain in place.

The manifests and source code own settings. The knowledge map links them and
does not duplicate runtime evidence. These edits describe the intended build
contract; build, HTTP, and deployment results remain pending in `edge-clock-3oh`.

Checked the rollback procedure against Cloudflare's public documentation and
Wrangler 4.93 help. Added the explicit version-ID command to the README.
