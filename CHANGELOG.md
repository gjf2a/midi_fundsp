# 0.2.0
  * `SynthMsg` objects can give note and velocity information if
  they correspond to `NOTE_ON` or `NOTE_OFF` MIDI messages.

# 0.1.7
  * Promoted `NUM_MIDI_VALUES` to be a public constant.

# 0.1.6
  * Previously, the sound output reclaimed sounds in the order they were activated. It now reuses unused sounds before reclaiming sounds still in use.

# 0.1.5
  * Updated to `fundsp 0.15`

# 0.1.4
  * Updated to `cpal 0.15` 

# 0.1.3
  * Updated to `fundsp 0.12` and `midir 0.9`

# 0.1.2
  * Added `semitone_from()`.

# 0.1.1
  * Updated to `fundsp 0.11`.
  * Added `adsr_organ()`, `moog_organ()`, `adsr_soft_saw()`, and `moog_soft_saw()` to `sounds.rs`.