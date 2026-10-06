# Reconnecting Halley UI

The original combined GPU/core migration is archived here as a porting reference.
That prototype branch was removed when the compositor was returned to `dev`. The
later
Smithay-independent 0.1.0 integration keeps GPU rendering and protocol buffer
handling in Halley, and shares only core text services and measured layout.
The source rollback did not change the installed/running executable.

## Preserve the compositor dependency

Halley requires Smithay revision `79bbed5e1199090d787115614847a79c76607181`.
The user identifies its blur support as the reason for this pin. Do not change it
to accommodate toolkit publication. An eventual Smithay 0.8 release requires a
separate compositor compatibility audit before migrating.

The chosen package boundary is a publishable core with no Smithay dependency.
The GPU prototype is preserved in `../../archive/` outside the published package.
A future adapter must be separately versioned and validated against the host.

## Saved migration

- `halley-migration.patch` contains the complete uncommitted compositor changes,
  including extracted renderers, notification layout, rendering comparison
  harness, the conservative-repaint test correction, and the attempted
  publication compatibility settings.
- `migration.json` records the exact base commit, branch, and changed files.
- `../verification/` contains earlier rendering and performance evidence. Those
  measurements do not validate the later interaction additions.

The archive is evidence and a porting reference, not a ready-to-apply integration
for an unreleased Smithay version. The toolkit was at version 0.1.0 locally.

## When integration resumes

1. Preserve the core/renderer boundary; separately authorize any GPU extraction.
2. Create a fresh Halley feature branch from the intended current base.
3. Review this patch against current code. Start with `git apply --check`;
   port individual renderer, text, and notification changes where needed.
4. Preserve Halley's dependency pin until a separate upgrade is authorized.
   Do not carry the archived `[patch.crates-io]` workaround or compatibility
   feature into the new integration without choosing that approach explicitly.
5. Resolve published dependencies from a clean checkout without sibling paths.
6. Run toolkit tests and real input/accessibility checks. Re-run Halley's pixel
   and reused-buffer correctness checks; retain conservative repaint policy.
7. Build and check performance separately. Source, installed binary, and running
   compositor are separate states; installation does not restart the session.

Use the earlier frozen frame baseline only in a matching font/driver/toolchain
environment. Once Halley changes substantially, record a new before-migration
baseline rather than treating historical fingerprints as universal.
