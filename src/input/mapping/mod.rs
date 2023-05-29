// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

pub mod korg_kaoss_dj;

#[must_use]
pub fn u7_to_slider(input: u8) -> super::Slider {
    let position = f32::from(input) / 127.0;
    super::Slider { position }
}

#[must_use]
pub fn u7_to_center_slider(input: u8) -> super::CenterSlider {
    let position = f32::from(input) * 2.0 / 127.0 - 1.0;
    super::CenterSlider { position }
}
