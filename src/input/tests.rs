// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use super::*;

#[test]
#[allow(clippy::float_cmp)]
fn pad_button_from_u7() {
    assert_eq!(
        PadButtonInput::MIN_PRESSURE,
        PadButtonInput::from_u7(0).pressure
    );
    assert_eq!(
        PadButtonInput::MAX_PRESSURE,
        PadButtonInput::from_u7(127).pressure
    );
}

#[test]
#[allow(clippy::float_cmp)]
fn pad_button_from_u14() {
    assert_eq!(
        PadButtonInput::MIN_PRESSURE,
        PadButtonInput::from_u14(0).pressure
    );
    assert_eq!(
        PadButtonInput::MAX_PRESSURE,
        PadButtonInput::from_u14(16383).pressure
    );
}

#[test]
#[allow(clippy::float_cmp)]
fn step_encoder_from_u7() {
    assert_eq!(0, StepEncoderInput::from_u7(0).delta);
    assert_eq!(1, StepEncoderInput::from_u7(1).delta);
    assert_eq!(63, StepEncoderInput::from_u7(63).delta);
    assert_eq!(-64, StepEncoderInput::from_u7(64).delta);
    assert_eq!(-1, StepEncoderInput::from_u7(127).delta);
}

#[test]
#[allow(clippy::float_cmp)]
fn step_encoder_from_u14() {
    assert_eq!(0, StepEncoderInput::from_u14(0).delta);
    assert_eq!(1, StepEncoderInput::from_u14(1).delta);
    assert_eq!(8191, StepEncoderInput::from_u14(8191).delta);
    assert_eq!(-8192, StepEncoderInput::from_u14(8192).delta);
    assert_eq!(-1, StepEncoderInput::from_u14(16383).delta);
}

#[test]
#[allow(clippy::float_cmp)]
fn slider_from_u7() {
    assert_eq!(SliderInput::MIN_POSITION, SliderInput::from_u7(0).position);
    assert_eq!(
        SliderInput::MAX_POSITION,
        SliderInput::from_u7(127).position
    );
}

#[test]
#[allow(clippy::float_cmp)]
fn slider_from_u14() {
    assert_eq!(SliderInput::MIN_POSITION, SliderInput::from_u14(0).position);
    assert_eq!(
        SliderInput::MAX_POSITION,
        SliderInput::from_u14(16383).position
    );
}

#[test]
#[allow(clippy::float_cmp)]
fn center_slider_from_u7() {
    assert_eq!(
        CenterSliderInput::MIN_POSITION,
        CenterSliderInput::from_u7(0).position
    );
    assert!(CenterSliderInput::MIN_POSITION < CenterSliderInput::from_u7(1).position);
    assert!(CenterSliderInput::CENTER_POSITION > CenterSliderInput::from_u7(63).position);
    assert_eq!(
        CenterSliderInput::CENTER_POSITION,
        CenterSliderInput::from_u7(64).position
    );
    assert!(CenterSliderInput::CENTER_POSITION < CenterSliderInput::from_u7(65).position);
    assert!(CenterSliderInput::MAX_POSITION > CenterSliderInput::from_u7(126).position);
    assert_eq!(
        CenterSliderInput::MAX_POSITION,
        CenterSliderInput::from_u7(127).position
    );
}

#[test]
#[allow(clippy::float_cmp)]
fn center_slider_from_u14() {
    assert_eq!(
        CenterSliderInput::MIN_POSITION,
        CenterSliderInput::from_u14(0).position
    );
    assert!(CenterSliderInput::MIN_POSITION < CenterSliderInput::from_u14(1).position);
    assert!(CenterSliderInput::CENTER_POSITION > CenterSliderInput::from_u14(8191).position);
    assert_eq!(
        CenterSliderInput::CENTER_POSITION,
        CenterSliderInput::from_u14(8192).position
    );
    assert!(CenterSliderInput::CENTER_POSITION < CenterSliderInput::from_u14(8193).position);
    assert!(CenterSliderInput::MAX_POSITION > CenterSliderInput::from_u14(16382).position);
    assert_eq!(
        CenterSliderInput::MAX_POSITION,
        CenterSliderInput::from_u14(16383).position
    );
}

#[test]
#[allow(clippy::float_cmp)]
fn slider_encoder_from_u7() {
    assert_eq!(0.0, SliderEncoderInput::from_u7(0).delta);
    assert_eq!(
        SliderEncoderInput::DELTA_PER_CW_REV,
        SliderEncoderInput::from_u7(63).delta
    );
    assert_eq!(
        SliderEncoderInput::DELTA_PER_CCW_REV,
        SliderEncoderInput::from_u7(64).delta
    );
}

#[test]
#[allow(clippy::float_cmp)]
fn slider_encoder_from_u14() {
    assert_eq!(0.0, SliderEncoderInput::from_u14(0).delta);
    assert_eq!(
        SliderEncoderInput::DELTA_PER_CW_REV,
        SliderEncoderInput::from_u14(8191).delta
    );
    assert_eq!(
        SliderEncoderInput::DELTA_PER_CCW_REV,
        SliderEncoderInput::from_u14(8192).delta
    );
}
