<!-- SPDX-FileCopyrightText: The djio authors -->
<!-- SPDX-License-Identifier: MPL-2.0 -->

# djio

[![Crates.io](https://img.shields.io/crates/v/djio.svg)](https://crates.io/crates/djio)
[![Docs.rs](https://docs.rs/djio/badge.svg)](https://docs.rs/djio)
[![Deps.rs](https://deps.rs/repo/github/uklotzde/djio/status.svg)](https://deps.rs/repo/github/uklotzde/djio)
[![Dependency audit](https://github.com/uklotzde/djio/actions/workflows/dependency-audit.yaml/badge.svg)](https://github.com/uklotzde/djio/actions/workflows/dependency-audit.yaml)
[![Continuous integration](https://github.com/uklotzde/djio/actions/workflows/test.yaml/badge.svg)](https://github.com/uklotzde/djio/actions/workflows/test.yaml)
[![License: MPL 2.0](https://img.shields.io/badge/License-MPL_2.0-brightgreen.svg)](https://opensource.org/licenses/MPL-2.0)

Application-independent interfaces for accessing hardware DJ controllers through MIDI or HID.

## Supported Controllers

### MIDI

- Denon DJ MC6000MK2
- Korg KAOSS DJ
- Pioneer DDJ-400

### HID

- Native Instruments TRAKTOR KONTROL S4MK3

## Examples

### Hotplugging of DJ MIDI controllers

```sh
RUST_LOG=debug cargo run --example midi-dj-controller-hotplug
```

## Credits

We have been inspired by and adopted some ideas from the following projects:

- [Ctlra - A C Library for Controller Support](https://github.com/openAVproductions/openAV-Ctlra)

## License

Licensed under the Mozilla Public License 2.0 (MPL-2.0) (see [MPL-2.0.txt](LICENSES/MPL-2.0.txt) or
<https://www.mozilla.org/MPL/2.0/>).

Permissions of this copyleft license are conditioned on making available source code of licensed
files and modifications of those files under the same license (or in certain cases, one of the GNU
licenses). Copyright and license notices must be preserved. Contributors provide an express grant of
patent rights. However, a larger work using the licensed work may be distributed under different
terms and without source code for files added in the larger work.

### Contribution

Any contribution intentionally submitted for inclusion in the work by you shall be licensed under
the Mozilla Public License 2.0 (MPL-2.0).

It is required to add the following header with the corresponding
[SPDX short identifier](https://spdx.dev/ids/) to the top of each file:

```rust
// SPDX-License-Identifier: MPL-2.0
```
