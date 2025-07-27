// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

//! Virtual DJ deck utilities.

use std::time::Duration;

use crate::{ButtonInput, CenterSliderInput, LedState, SliderInput};

pub const PLAYBACK_RATE_DEFAULT: f32 = 1.0;

pub const PLAYBACK_RATE_PAUSED: f32 = 0.0;

pub const TEMPO_RANGE_MAX_DEFAULT: f32 = 0.08; // +8%

pub const TEMPO_RANGE_MIN_DEFAULT: f32 = -TEMPO_RANGE_MAX_DEFAULT; // symmetric

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Position {
    pub offset_secs: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Cue {
    pub position: Position,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Playable {
    pub play_state: PlayState,

    /// Duration of the media
    ///
    /// `None` if unlimited or unknown in advance.
    pub duration: Option<Duration>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TempoInput {
    pub range_min: f32,
    pub range_max: f32,
    pub center_slider: CenterSliderInput,
}

impl TempoInput {
    #[must_use]
    pub fn playback_rate(&self) -> f32 {
        let range_center = self.range_min.midpoint(self.range_max);
        PLAYBACK_RATE_DEFAULT
            + self
                .center_slider
                .map_position_linear(self.range_min, range_center, self.range_max)
    }
}

impl Default for TempoInput {
    fn default() -> Self {
        Self {
            range_min: TEMPO_RANGE_MIN_DEFAULT,
            range_max: TEMPO_RANGE_MAX_DEFAULT,
            center_slider: CenterSliderInput {
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
    /// - Nominal tempo (default): 1.0
    /// - Slowed down by 6%: 0.94
    /// - Sped up by 6%: 1.06
    /// - Halted or paused: 0.0
    /// - Reversed with nominal tempo: -1.0
    ///
    /// If the playback rate affects the pitch depends on the value of
    /// `pitch_semitones`.
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

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Player {
    /// Cue
    pub cue: Cue,

    /// Playback parameters
    pub playback_params: PlaybackParams,
}

/// [`Player`] with all fields optional
///
/// Fields that are `None` will not be updated.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
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
#[expect(missing_debug_implementations)]
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
    /// Reads the current playhead.
    #[must_use]
    fn read_playhead(&self) -> Option<Playhead>;

    /// Sets the playhead position.
    ///
    /// The playhead position might not become effective immediately,
    /// i.e. [`Self::read_playhead()`] could still return the old position
    /// after returning from this method.
    fn set_playhead_position(&mut self, position: Position);

    /// Updates selected [`Player`] properties.
    ///
    /// If `with_playhead` is `Some`, then this value should be used instead
    /// of reading the current value when needed.
    fn update_player(&mut self, update_player: UpdatePlayer, with_playhead: Option<Playhead>);
}
