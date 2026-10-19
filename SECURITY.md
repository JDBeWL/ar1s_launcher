# Security Policy

## Supported versions

Security fixes are applied to the latest release and the `main` branch. Older
builds are not maintained.

## Reporting a vulnerability

Please **do not open a public issue**. Use GitHub's private vulnerability
reporting (repository → _Security_ → _Report a vulnerability_). Include the
affected version, reproduction steps and any log excerpt that helps. You can
expect an initial response within about a week.

## Secrets policy

The launcher talks to third-party services that require credentials, so the
following rules apply to every contribution:

- **No secrets in the repository.** API keys, tokens and passwords must never
  appear in source files, tests, fixtures, screenshots or commit messages.
- **No secrets in the build.** Never use `env!()` / `option_env!()` for a key,
  and never put one in `tauri.conf.json`. A key compiled into the binary can be
  extracted with a string search.
- **No secrets in the frontend.** Values prefixed with `VITE_` are bundled into
  the shipped JavaScript. All authenticated requests happen in the Rust backend;
  the webview never receives a key.
- **User-supplied keys stay local.** Keys entered in Settings are stored outside
  the repository, in the user's own profile, and are never uploaded anywhere.
- **Stored tokens live in one restricted file.** The Microsoft token is written
  into `ar1s.json` alongside the other settings, and that file is restricted to
  the current user on every save (0600 on Unix, a tightened DACL on Windows).
  Diagnostic bundles always strip the token fields.
- **Environment variables are a development convenience only.** `CURSEFORGE_API_KEY`
  (see `.env.example`) is read as a fallback in debug builds and is ignored in
  release builds.
- **Logs and diagnostics stay clean.** Do not log request headers or tokens, and
  do not dump the process environment into diagnostic bundles.

## If a key leaks

Revoke and reissue it in the provider's console immediately. Rewriting git
history is not sufficient: forks, caches and secret-scanning notifications may
already contain the value.

## Scope

This policy covers the launcher, its release artifacts and the scripts in this
repository.
