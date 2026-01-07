# 0.6.8
  * Updated to fundsp 0.22.0

# 0.6.7
  * Updated to cpal 0.17.0

# 0.6.6
  * Updated to midir 0.10.3

# 0.6.5
  * Added Xylophone (thanks @xenacool!)

# 0.6.4
  * Updated to anyhow 1.0.100 and midi-msg 0.8.1

# 0.6.3
  * Added Acoustic Grand Piano (thanks @xenacool!)

# 0.6.2
  * Updated to cpal 0.16.0
  * Specified full 3-digit dependencies for all.
  * Updated to Rust 2024 edition.

# 0.6.1
  * Updated to midi-msg 0.8.0.

# 0.6.0
  * Updated to fundsp 0.20.0.
  * Since 0.20.0 has a significantly backward-incompatible API, all references
    to `bin_op()` are now `binary()`, `stack_op()` are now `stack()`, and `pipe_op()` are now `pipe()`.

# 0.5.3
  * Updated to fundsp 0.19.1

# 0.5.2
  * Added `start_midi_output_thread()`, enabling users to rely solely on `MidiMsg` objects
  rather than using `SynthMsg` objects. The `midi_only_demo` example demonstrates using
  `start_midi_input_thread()` and `start_midi_output_thread()` together.

# 0.5.1
  * Added documentation for `start_midi_input_thread()`

# 0.5.0
  * Added `start_midi_input_thread()`, enabling users to set up an input thread that enqueues `MidiMsg` objects rather than `SynthMsg` objects. The `stereo_demo` example was updated to demonstrate its use.

# 0.4.1
  * Updated to `fundsp 0.18.2`

# 0.4.0
  * Updated to `fundsp 0.18.1`
  * Replaced most `f64` values with `f32` values, to reflect changes to `fundsp`.
    * This is not a backwards-compatible update, but it should be easy to fix.

# 0.3.6
  * Updated to `midir 0.10` and `midi-msg 0.7`

# 0.3.5
  * Updated to `fundsp 0.17.1`.

# 0.3.4
  * Updated to `fundsp 0.17.0`.

# 0.3.3
  * Updated to `midi-msg 0.6.1`.

# 0.3.2
  * Updated to `fundsp 0.16.0`.

# 0.3.1
  * Updated `README.md` to be consistent with the new version of `start_output_thread()`.

# 0.3.0
  * Updated to `midi-msg 0.5.0`.
  * [Disabled `files` feature in `fundsp`](https://github.com/gjf2a/midi_fundsp/pull/3), as this library does not open any files.
  * Changed API for `start_output_thread()`. 
    * Instead of relying on the `AtomicCell` variable `quit` to determine when to stop, it now relies upon receiving a MIDI `SystemReset` message. 
    * That message will be sent by `start_input_thread()` when its `quit` variable is set to `true`. It will reset `quit` to `false` once it has finished running.
    * This change fixes a [bug](https://github.com/gjf2a/midi_fundsp/issues/2) which was caused by the output thread in [`choice_demo.rs`](https://github.com/gjf2a/midi_fundsp/blob/master/examples/choice_demo.rs) failing to exit, a consequence of an unpredictable sequence of when the `reset` variable would be changed back to `false`.

# 0.2.1
  * A `NoteOn` message with a velocity of zero is treated as a `NoteOff` message. Some devices implement `NoteOff` in this way, and this change supports them.

# 0.2.0
  * `SynthMsg` objects can give note and velocity information if they correspond to `NOTE_ON` or `NOTE_OFF` MIDI messages.
  * `note_velocity_demo.rs` is an example that intercepts the MIDI messages and prints the note and velocity values.
  * `stereo_demo.rs` has been modified to make use of this new feature as well.

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