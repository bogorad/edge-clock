# Emscripten reference update

Bead `edge-clock-3oh` tracks the migration. Added a scoped Emscripten binding
reference and linked it from the older bindgen guides. The older `cdylib` and
test examples are now clearly separate from the project's build and debug
contract. This prevents the generic guide from restoring the previous loader
or bypassing the live HTTP gate.

The reference describes source ownership and expected verification. It does
not claim that the new artifact has built or been deployed.
