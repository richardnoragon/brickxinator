# Third-party dependency review

Review date: 2026-11-01. Project code is GPL-3.0-or-later. Dependency license
declarations below come from pinned package metadata and registry declarations; upstream packages carry
their own copyright and license texts. Dependencies and tools are not vendored.

## Advisory checks

- cargo-audit 0.22.2: 137 lockfile packages; no vulnerabilities or informational warnings.
- RustSec database commit: `2295466c4e9df26647fd67d8fa1c705ac2fc7f81` (2026-10-30).
- Fresh npm audit: zero production findings; two low-severity development findings.
- `deepmerge` 4.3.1 and its dependent `@sveltejs/vite-plugin-svelte` are affected by
  [GHSA-gcqx-w48m-w2h4](https://github.com/advisories/GHSA-gcqx-w48m-w2h4).
  The tool reports no fix available for this pinned dependency path. It merges
  trusted build configuration; collection data is not passed into that build tool.
  This is a documented build-tool finding for candidate review.

The built frontend source map contains no deepmerge source. See ALPHA_STATUS.md
for runtime/bundle evidence and release gates.

## Rust workspace dependency declarations

| Package | Version | Declared license |
| --- | --- | --- |
| android_system_properties | 0.1.6 | MIT OR Apache-2.0 |
| atomic-waker | 1.1.2 | Apache-2.0 OR MIT |
| autocfg | 1.5.1 | Apache-2.0 OR MIT |
| axum | 0.8.9 | MIT |
| axum-core | 0.5.6 | MIT |
| bitflags | 2.13.2 | MIT OR Apache-2.0 |
| block-buffer | 0.10.4 | MIT OR Apache-2.0 |
| brickxinator-app | 0.1.0-alpha.1 | GPL-3.0-or-later |
| brickxinator-core | 0.1.0-alpha.1 | GPL-3.0-or-later |
| brickxinator-infra | 0.1.0-alpha.1 | GPL-3.0-or-later |
| bumpalo | 3.20.3 | MIT OR Apache-2.0 |
| bytes | 1.12.1 | MIT |
| cc | 1.10.0 | MIT OR Apache-2.0 |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 |
| cfg_aliases | 0.2.2 | MIT |
| chrono | 0.4.45 | MIT OR Apache-2.0 |
| core-foundation-sys | 0.8.7 | MIT OR Apache-2.0 |
| core_detect | 1.0.0 | MIT/Apache-2.0 |
| cpufeatures | 0.2.17 | MIT OR Apache-2.0 |
| crc32fast | 1.5.3 | MIT OR Apache-2.0 |
| crypto-common | 0.1.7 | MIT OR Apache-2.0 |
| digest | 0.10.7 | MIT OR Apache-2.0 |
| encoding_rs | 0.8.44 | (Apache-2.0 OR MIT) AND BSD-3-Clause |
| equivalent | 1.0.3 | Apache-2.0 OR MIT |
| errno | 0.3.14 | MIT OR Apache-2.0 |
| fallible-iterator | 0.3.0 | MIT/Apache-2.0 |
| fallible-streaming-iterator | 0.1.9 | MIT/Apache-2.0 |
| fastrand | 2.5.0 | Apache-2.0 OR MIT |
| find-msvc-tools | 0.1.15 | MIT OR Apache-2.0 |
| flate2 | 1.1.10 | MIT OR Apache-2.0 |
| foldhash | 0.2.0 | Zlib |
| form_urlencoded | 1.2.2 | MIT OR Apache-2.0 |
| futures-core | 0.3.34 | MIT OR Apache-2.0 |
| futures-sink | 0.3.34 | MIT OR Apache-2.0 |
| futures-task | 0.3.34 | MIT OR Apache-2.0 |
| futures-util | 0.3.34 | MIT OR Apache-2.0 |
| generic-array | 0.14.7 | MIT |
| getrandom | 0.4.3 | MIT OR Apache-2.0 |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 |
| hashlink | 0.12.2 | MIT OR Apache-2.0 |
| http | 1.5.0 | MIT OR Apache-2.0 |
| http-body | 1.1.0 | MIT |
| http-body-util | 0.1.5 | MIT |
| http-range-header | 0.4.2 | MIT |
| httparse | 1.10.1 | MIT OR Apache-2.0 |
| httpdate | 1.0.3 | MIT OR Apache-2.0 |
| hyper | 1.12.0 | MIT |
| hyper-util | 0.1.21 | MIT |
| iana-time-zone | 0.1.65 | MIT OR Apache-2.0 |
| iana-time-zone-haiku | 0.1.2 | MIT OR Apache-2.0 |
| indexmap | 2.14.2 | Apache-2.0 OR MIT |
| itoa | 1.0.18 | MIT OR Apache-2.0 |
| js-sys | 0.3.107 | MIT OR Apache-2.0 |
| libc | 0.2.191 | MIT OR Apache-2.0 |
| libsqlite3-sys | 0.39.1 | MIT |
| linux-raw-sys | 0.12.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| lock_api | 0.4.14 | MIT OR Apache-2.0 |
| log | 0.4.35 | MIT OR Apache-2.0 |
| matchit | 0.8.4 | MIT AND BSD-3-Clause |
| memchr | 2.8.3 | Unlicense OR MIT |
| mime | 0.3.17 | MIT OR Apache-2.0 |
| mime_guess | 2.0.5 | MIT |
| mio | 1.2.4 | MIT |
| multer | 3.1.0 | MIT |
| multiversion_no_op | 1.0.0 | Apache-2.0 OR MIT |
| nix | 0.31.3 | MIT |
| num-traits | 0.2.19 | MIT OR Apache-2.0 |
| once_cell | 1.21.4 | MIT OR Apache-2.0 |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT |
| pkg-config | 0.3.34 | MIT OR Apache-2.0 |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |
| quote | 1.0.47 | MIT OR Apache-2.0 |
| r-efi | 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later |
| redox_syscall | 0.5.18 | MIT |
| rsqlite-vfs | 0.2.0 | MIT |
| rusqlite | 0.41.1 | MIT |
| rustix | 1.1.6 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| rustversion | 1.0.23 | MIT OR Apache-2.0 |
| ryu | 1.0.23 | Apache-2.0 OR BSL-1.0 |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 |
| serde | 1.0.229 | MIT OR Apache-2.0 |
| serde_core | 1.0.229 | MIT OR Apache-2.0 |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 |
| serde_path_to_error | 0.1.20 | MIT OR Apache-2.0 |
| serde_urlencoded | 0.7.1 | MIT/Apache-2.0 |
| sha2 | 0.10.9 | MIT OR Apache-2.0 |
| shlex | 2.0.1 | MIT OR Apache-2.0 |
| signal-hook-registry | 1.4.8 | MIT OR Apache-2.0 |
| simd-adler32 | 0.3.10 | MIT |
| simdutf8 | 0.1.5 | MIT OR Apache-2.0 |
| slab | 0.4.12 | MIT |
| smallvec | 1.16.4 | MIT OR Apache-2.0 |
| socket2 | 0.6.5 | MIT OR Apache-2.0 |
| spin | 0.9.9 | MIT |
| sqlite-wasm-rs | 0.6.2 | MIT |
| syn | 2.0.119 | MIT OR Apache-2.0 |
| syn | 3.0.9 | MIT OR Apache-2.0 |
| sync_wrapper | 1.0.2 | Apache-2.0 |
| tempfile | 3.27.0 | MIT OR Apache-2.0 |
| thiserror | 2.0.21 | MIT OR Apache-2.0 |
| thiserror-impl | 2.0.21 | MIT OR Apache-2.0 |
| tokio | 1.54.1 | MIT |
| tokio-macros | 2.7.2 | MIT |
| tokio-util | 0.7.20 | MIT |
| tower | 0.5.3 | MIT |
| tower-http | 0.6.11 | MIT |
| tower-layer | 0.3.3 | MIT |
| tower-service | 0.3.3 | MIT |
| tracing | 0.1.44 | MIT |
| tracing-core | 0.1.36 | MIT |
| typed-path | 0.13.0 | MIT OR Apache-2.0 |
| typenum | 1.20.1 | MIT OR Apache-2.0 |
| unicase | 2.10.0 | MIT OR Apache-2.0 |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| uuid | 1.26.1 | Apache-2.0 OR MIT |
| vcpkg | 0.2.15 | MIT/Apache-2.0 |
| version_check | 0.9.5 | MIT/Apache-2.0 |
| wasi | 0.11.1+wasi-snapshot-preview1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| wasm-bindgen | 0.2.130 | MIT OR Apache-2.0 |
| wasm-bindgen-macro | 0.2.130 | MIT OR Apache-2.0 |
| wasm-bindgen-macro-support | 0.2.130 | MIT OR Apache-2.0 |
| wasm-bindgen-shared | 0.2.130 | MIT OR Apache-2.0 |
| windows-core | 0.62.2 | MIT OR Apache-2.0 |
| windows-implement | 0.60.2 | MIT OR Apache-2.0 |
| windows-interface | 0.59.3 | MIT OR Apache-2.0 |
| windows-link | 0.2.1 | MIT OR Apache-2.0 |
| windows-result | 0.4.1 | MIT OR Apache-2.0 |
| windows-strings | 0.5.1 | MIT OR Apache-2.0 |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 |
| zip | 9.2.1 | MIT |
| zlib-rs | 0.6.8 | Zlib |
| zmij | 1.0.23 | MIT |
| zopfli | 0.8.4 | Apache-2.0 |

## Frontend dependency declarations

| Package | Version | Declared license | Scope |
| --- | --- | --- | --- |
| @jridgewell/gen-mapping | 0.3.13 | MIT | Runtime |
| @jridgewell/remapping | 2.3.5 | MIT | Runtime |
| @jridgewell/resolve-uri | 3.1.2 | MIT | Runtime |
| @jridgewell/sourcemap-codec | 1.6.0 | MIT | Runtime |
| @jridgewell/trace-mapping | 0.3.31 | MIT | Runtime |
| @oxc-project/types | 0.156.0 | MIT | Development |
| @playwright/test | 1.64.1 | Apache-2.0 | Development |
| @rolldown/binding-android-arm-eabi | 1.2.16 | MIT | Development |
| @rolldown/binding-android-arm64 | 1.2.16 | MIT | Development |
| @rolldown/binding-darwin-arm64 | 1.2.16 | MIT | Development |
| @rolldown/binding-darwin-x64 | 1.2.16 | MIT | Development |
| @rolldown/binding-freebsd-x64 | 1.2.16 | MIT | Development |
| @rolldown/binding-linux-arm-gnueabihf | 1.2.16 | MIT | Development |
| @rolldown/binding-linux-arm64-gnu | 1.2.16 | MIT | Development |
| @rolldown/binding-linux-arm64-musl | 1.2.16 | MIT | Development |
| @rolldown/binding-linux-ppc64-gnu | 1.2.16 | MIT | Development |
| @rolldown/binding-linux-s390x-gnu | 1.2.16 | MIT | Development |
| @rolldown/binding-linux-x64-gnu | 1.2.16 | MIT | Development |
| @rolldown/binding-linux-x64-musl | 1.2.16 | MIT | Development |
| @rolldown/binding-openharmony-arm64 | 1.2.16 | MIT | Development |
| @rolldown/binding-win32-arm64-msvc | 1.2.16 | MIT | Development |
| @rolldown/binding-win32-x64-msvc | 1.2.16 | MIT | Development |
| @rolldown/pluginutils | 1.0.1 | MIT | Development |
| @sveltejs/acorn-typescript | 1.0.13 | MIT | Runtime |
| @sveltejs/load-config | 0.2.3 | MIT | Development |
| @sveltejs/vite-plugin-svelte | 7.3.1 | MIT | Development |
| @types/estree | 1.0.10 | MIT | Runtime |
| acorn | 8.19.0 | MIT | Runtime |
| aria-query | 5.3.1 | Apache-2.0 | Runtime |
| axobject-query | 4.1.0 | Apache-2.0 | Runtime |
| chokidar | 4.0.3 | MIT | Development |
| clsx | 2.1.1 | MIT | Runtime |
| deepmerge | 4.3.1 | MIT | Development |
| detect-libc | 2.1.2 | Apache-2.0 | Development |
| devalue | 5.9.4 | MIT | Runtime |
| esm-env | 1.2.2 | MIT | Runtime |
| esrap | 2.4.0 | MIT | Runtime |
| fdir | 6.5.0 | MIT | Development |
| fsevents | 2.3.3 | MIT | Development |
| is-reference | 3.0.3 | MIT | Runtime |
| lightningcss | 1.33.0 | MPL-2.0 | Development |
| lightningcss-android-arm64 | 1.33.0 | MPL-2.0 | Development |
| lightningcss-darwin-arm64 | 1.33.0 | MPL-2.0 | Development |
| lightningcss-darwin-x64 | 1.33.0 | MPL-2.0 | Development |
| lightningcss-freebsd-x64 | 1.33.0 | MPL-2.0 | Development |
| lightningcss-linux-arm-gnueabihf | 1.33.0 | MPL-2.0 | Development |
| lightningcss-linux-arm64-gnu | 1.33.0 | MPL-2.0 | Development |
| lightningcss-linux-arm64-musl | 1.33.0 | MPL-2.0 | Development |
| lightningcss-linux-x64-gnu | 1.33.0 | MPL-2.0 | Development |
| lightningcss-linux-x64-musl | 1.33.0 | MPL-2.0 | Development |
| lightningcss-win32-arm64-msvc | 1.33.0 | MPL-2.0 | Development |
| lightningcss-win32-x64-msvc | 1.33.0 | MPL-2.0 | Development |
| locate-character | 3.0.0 | MIT | Runtime |
| magic-string | 1.4.5 | MIT | Development |
| mri | 1.2.0 | MIT | Development |
| nanoid | 3.3.20 | MIT | Development |
| obug | 2.2.1 | MIT | Development |
| picocolors | 1.1.1 | ISC | Development |
| picomatch | 4.0.7 | MIT | Development |
| playwright | 1.64.1 | Apache-2.0 | Development |
| playwright-core | 1.64.1 | Apache-2.0 | Development |
| postcss | 8.5.32 | MIT | Development |
| prettier | 3.9.10 | MIT | Development |
| prettier-plugin-svelte | 4.1.1 | MIT | Development |
| readdirp | 4.1.2 | MIT | Development |
| rolldown | 1.2.16 | MIT | Development |
| sade | 1.8.1 | MIT | Development |
| source-map-js | 1.2.2 | BSD-3-Clause | Development |
| svelte | 5.57.3 | MIT | Runtime |
| svelte-check | 4.7.7 | MIT | Development |
| magic-string | 0.30.21 | MIT | Runtime |
| tinyglobby | 0.2.17 | MIT | Development |
| typescript | 6.0.3 | Apache-2.0 | Development |
| vite | 8.3.5 | MIT | Development |
| vitefu | 1.1.3 | MIT | Development |
| zimmerframe | 1.1.5 | MIT | Runtime |
