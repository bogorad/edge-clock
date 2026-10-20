# Asset HTTP check

The new runtime served the home, theme, and CSS routes. The HTTP gate then
rejected the asset service's canonical redirect from `@` to `%40` in the HTMX
path. The redirect target returned HTTP 200 and the same SHA-256 as the pinned
local HTMX file: `449317ade7881e949510db614991e195c3a099c4c791c24dacec55f9f4a2a452`.

The script check now accepts that exact local redirect and checks the final
status, content type, and file hash. It does not follow arbitrary URLs. The
Worker adapter and browser assets did not need a change. Full live validation
remains pending in `edge-clock-3oh`.
