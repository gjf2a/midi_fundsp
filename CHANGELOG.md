# 0.3.0
  * Updated to `midi-msg 0.5.0`
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