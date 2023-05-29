// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

pub mod korg_kaoss_dj;

#[must_use]
pub fn u7_be_to_u14(hi: u8, lo: u8) -> u16 {
    u16::from(hi) << 7 | u16::from(lo)
}

#[must_use]
pub fn u7_to_slider(input: u8) -> super::Slider {
    let position = f32::from(input) / 127.0;
    super::Slider { position }
}

#[must_use]
pub fn u7_to_slider_encoder(input: u8) -> super::SliderEncoder {
    let delta = if input < 64 {
        f32::from(input) / 63.0
    } else {
        f32::from(input.wrapping_sub(128) as i8) / 64.0
    };
    super::SliderEncoder { delta }
}

#[must_use]
pub fn u14_to_slider(input: u16) -> super::Slider {
    let position = f32::from(input) / 16383.0;
    super::Slider { position }
}

#[must_use]
pub fn u7_to_center_slider(input: u8) -> super::CenterSlider {
    let position = f32::from(input) * 2.0 / 127.0 - 1.0;
    super::CenterSlider { position }
}

#[must_use]
pub fn u14_to_center_slider(input: u16) -> super::CenterSlider {
    let position = f32::from(input) * 2.0 / 16383.0 - 1.0;
    super::CenterSlider { position }
}
