// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

#[cfg(feature = "midi")]
pub mod korg_kaoss_dj;

#[cfg(feature = "midi")]
pub mod pioneer_ddj_400;

#[cfg(feature = "hid")]
pub mod ni_traktor_s4mk3;
