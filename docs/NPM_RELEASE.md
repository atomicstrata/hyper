# Browser SDK npm release

The browser package is `@atomicstrata/hyper` in `web/`. It is ESM JavaScript with
TypeScript declarations and prebuilt WebGPU and WebGL2 artifacts, with no runtime
npm dependencies or consumer Rust requirement. Its initial version is `0.2.0`.
Preparing a tarball or updating a PR does not publish a package.

## Build and verify a candidate

Use Node.js 24, repository Rust 1.89 and the matching wasm-bindgen CLI listed in
the [browser guide](BROWSER.md), plus cargo-about 0.9.2 for dependency notices:

```sh
cargo install cargo-about --version 0.9.2 --locked --features cli
npm ci --prefix web
npm test --prefix web
npm run build --prefix web
npm run test:package --prefix web
# Real hardware browser, both forced backends:
HYPER_BROWSER='/path/to/chromium' npm run test:package --prefix web -- --browser
cd web
npm pack
```

The package test installs the actual tarball into a clean consumer, imports its
ESM entry, checks TypeScript under NodeNext and Bundler resolution, bundles the
consumer and uses the shipped asset-copy CLI. With `--browser`, it renders from
a subpath using only the selected backend. `HYPER_SOFTWARE_RENDERING=1` runs
only a software WebGL2 smoke test; CI labels it separately. Existing hardware
verification remains necessary before claiming support for another platform.

`prepack` refuses missing, mismatched or invalid renderer artifacts. The
publish allowlist includes the API, declarations, renderer manifest and files,
asset-copy CLI, validation tools, README, licenses and NOTICE. Full notices for
the locked WASM dependencies and original bundled font licenses are generated
with cargo-about and shipped in `dist/licenses/`; the asset-copy CLI retains
them beside hosted renderers. Tests, demo,
Rust source and development dependencies are excluded. Current tarballs are
about 24.4 MiB compressed, containing both approximately 50 MiB WASM backends.

The Browser SDK workflow uploads `hyper-browser-npm` (the candidate `.tgz`) and
`hyper-browser-sdk` (the static SDK and demo). It never publishes from a PR or
ordinary branch push. Obsolete PR workflow runs are cancelled when a new
revision is pushed.

## npm account and trusted publishing setup

An npm account authorized to publish public packages under `@atomicstrata` is
required. Registry name availability does not establish scope ownership.
For a new package, a maintainer may need to bootstrap its first publication
interactively, using the verified CI tarball:

```sh
npm login
npm publish atomicstrata-hyper-0.2.0.tgz --access public
```

This is a release action, to be run only after the release decision. After the
package exists, configure its npm trusted publisher for GitHub owner
`atomicstrata`, repository `hyper`, workflow filename `npm.yml`, and environment
`npm`. Configure that GitHub environment's reviewers and release restrictions
as desired. No long-lived npm token is needed by the workflow. npm trusted
publishing requires npm CLI 11.5.1+ and Node.js 22.14+; the workflow uses Node 24.
See [npm's trusted publisher guide](https://docs.npmjs.com/trusted-publishers/).

## Subsequent approved releases

Update the npm version and lockfile in a PR, rebuild, and verify the candidate.
The version belongs to this package; it need not track Python releases. After
merging and deciding to release, create and push `npm-v<version>` at that commit:

```sh
git tag npm-v0.2.1
git push origin npm-v0.2.1
```

The npm release workflow requires the tag to match `web/package.json`, reuses
the full browser build/package verification, and publishes its exact tarball
through the `npm` environment with OIDC and provenance. It does not create a
tag or change versions for you. Existing npm versions cannot be overwritten;
fix release defects with a new version.

Hosts should copy assets again after upgrades and retain older hashed files
long enough for cached pages to finish. Point each deployed app at its version's
manifest; do not mix a new JavaScript SDK with an older renderer manifest.
