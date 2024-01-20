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
        playhead_on_cue: bool,
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
            PlayState::Paused {
                playhead_on_cue: true,
            }
            | PlayState::Previewing { .. }
            | PlayState::Playing => LedState::On,
            PlayState::Paused {
                playhead_on_cue: false,
            } => LedState::BlinkFast,
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

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Playhead {
    pub position: Position,
    pub is_playing: bool,
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
    /// Controls the tempo when the media is playing.
    ///
    /// A value of 1.0 means normal playback speed. A value of 0.0 means halt.
    /// If the playback rate is negative, the media will be played backwards.
    ///
    /// If the playback rate affects the pitch, depends on `pitch_semitones`.
    pub rate: f32,

    /// Pitch
    ///
    /// `None` if disabled, i.e. changing the tempo implicitly changes the pitch.
    ///
    /// `Some(0)` will preserve the original pitch independent of the tempo. i.e.
    /// independent of the playback rate.
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

    /// Playback parameters
    pub playback_params: PlaybackParams,
}

/// [`Player`] with all fields optional
///
/// Fields that are `None` will not be updated.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UpdatePlayer {
    pub cue: Option<Cue>,
    pub playback_params: Option<PlaybackParams>,
}

/// Deck inputs
#[derive(Debug, Clone, Copy)]
pub enum Input {
    Cue(ButtonInput),
    PlayPause(ButtonInput),
    Sync(ButtonInput),
    Position(SliderInput),
    RelativeTempo(CenterSliderInput),
    PitchSemitones(Option<i8>),
}

#[cfg(feature = "observables")]
#[derive(Default)]
#[allow(missing_debug_implementations)]
pub struct Observables {
    pub playable: discro::Publisher<Option<Playable>>,
    pub player: discro::Publisher<Player>,
}

#[cfg(feature = "observables")]
impl Observables {
    pub fn on_playhead_changed(&mut self, playhead_on_cue: bool) {
        self.playable.modify(|playable| {
            let Some(playable) = playable.as_mut() else {
                return false;
            };
            match playable.play_state {
                PlayState::Paused {
                    playhead_on_cue: paused_on_cue,
                } => {
                    if playhead_on_cue != paused_on_cue {
                        playable.play_state = PlayState::Paused { playhead_on_cue };
                        return true;
                    }
                }
                PlayState::Ended => {
                    playable.play_state = PlayState::Paused { playhead_on_cue };
                    return true;
                }
                PlayState::Playing | PlayState::Previewing { .. } => (),
            }
            // Unchanged
            false
        });
    }
}

pub trait Adapter {
    /// Read the current playhead
    #[must_use]
    fn read_playhead(&self) -> Option<Playhead>;

    /// Set the playhead position
    ///
    /// The playhead position might not become effective immediately,
    /// i.e. [`Self::read_playhead()`] could still return the old position
    /// after returning from this method.
    fn set_playhead_position(&mut self, position: Position);

    /// Update selected [`Player`] properties
    ///
    /// If `playhead` is `Some`, then this value should be used instead
    /// of reading the current value.
    fn update_player(&mut self, playhead: Option<Playhead>, update_player: UpdatePlayer);
}
