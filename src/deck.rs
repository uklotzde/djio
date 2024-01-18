// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

//! Virtual DJ deck utilities.

use std::time::Duration;

use crate::{ButtonInput, CenterSliderInput, LedState, SliderInput};

pub const PLAYBACK_RATE_DEFAULT: f32 = 1.0;

pub const PLAYBACK_RATE_PAUSED: f32 = 0.0;

/// +/- 8%
pub const TEMPO_RANGE_DEFAULT: f32 = 0.08;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Position {
    pub offset_secs: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Cue {
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlayState {
    /// Paused
    Paused {
        on_cue: bool,
    },
    /// Previewing while (hot) cue is pressed
    Previewing {
        /// The cue or hot cue that is being previewed
        cue: Cue,
    },
    /// Playing
    Playing,
    // Ended
    Ended,
}

impl PlayState {
    #[must_use]
    pub const fn pioneer_cue_led_state(&self) -> LedState {
        match self {
            PlayState::Paused { on_cue: true }
            | PlayState::Previewing { .. }
            | PlayState::Playing => LedState::On,
            PlayState::Paused { on_cue: false } => LedState::BlinkFast,
            PlayState::Ended => LedState::Off,
        }
    }

    #[must_use]
    pub const fn pioneer_playpause_led_state(&self) -> LedState {
        match self {
            PlayState::Playing => LedState::On,
            PlayState::Paused { .. } | PlayState::Previewing { .. } => LedState::BlinkSlow,
            PlayState::Ended => LedState::Off,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Playable {
    pub play_state: PlayState,

    /// Duration of the media
    ///
    /// `None` if unlimited or unknown in advance.
    pub duration: Option<Duration>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tempo {
    pub range: f32,
    pub input: CenterSliderInput,
}

impl Tempo {
    #[must_use]
    pub fn playback_rate(self) -> f32 {
        PLAYBACK_RATE_DEFAULT + self.input.map_position_linear(-self.range, 0.0, self.range)
    }
}

impl Default for Tempo {
    fn default() -> Self {
        Self {
            range: TEMPO_RANGE_DEFAULT,
            input: CenterSliderInput {
                position: CenterSliderInput::CENTER_POSITION,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaybackParams {
    /// Playback rate
    ///
    /// A value of 1.0 means normal playback speed. A value of 0.0 means paused.
    ///
    /// If the playback rate is negative, the media will be played backwards.
    ///
    /// If the playback rate affects the pitch, depends on `pitch_semitones`.
    pub rate: f32,

    /// Pitch
    ///
    /// `None` if disabled, i.e. changing the tempo implicitly changes the pitch.
    ///
    /// `Some(0)` will preserve the original pitch.
    pub pitch_semitones: Option<i8>,
}

impl Default for PlaybackParams {
    fn default() -> Self {
        Self {
            rate: PLAYBACK_RATE_DEFAULT,
            pitch_semitones: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Player {
    /// Cue
    pub cue: Cue,

    /// Tempo
    pub tempo: Tempo,

    pub playback_params: PlaybackParams,
}

/// `Player` with all fields optional
///
/// Fields that are `None` will not be updated.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UpdatePlayer {
    pub cue: Option<Cue>,
    pub tempo: Option<Tempo>,
    pub playback_params: Option<PlaybackParams>,
}

/// Deck inputs
#[derive(Debug, Clone, Copy)]
pub enum Input {
    Cue(ButtonInput),
    PlayPause(ButtonInput),
    Sync(ButtonInput),
    Position(SliderInput),
    Tempo(CenterSliderInput),
}

#[cfg(feature = "observables")]
#[derive(Default)]
#[allow(missing_debug_implementations)]
pub struct Observables {
    pub playable: discro::Publisher<Option<Playable>>,
    pub player: discro::Publisher<Player>,
}

pub trait Device {
    /// Get the playhead position
    #[must_use]
    fn playhead(&self) -> Position;

    /// Set the playhead position
    fn set_playhead(&mut self, position: Position);

    /// Update selected [`Player`] fields
    fn update_player(&mut self, update_player: UpdatePlayer);
}
