# midi_fundsp

## Live performance synthesizer library

This crate assembles and integrates code from the [midir](https://crates.io/crates/midir), 
[midi-msg](https://crates.io/crates/midi-msg), and [cpal](https://crates.io/crates/cpal) 
crates to enable the easy creation of live synthesizer software using 
[fundsp](https://crates.io/crates/fundsp) for sound synthesis.

Using the crate involves setting up the following:
* An input thread to monitor MIDI events
* An output thread generating sounds that correspond to those events
* A table of [fundsp](https://crates.io/crates/fundsp) sounds for the output thread to employ
* A [`SegQueue`](https://docs.rs/crossbeam/latest/crossbeam/queue/struct.SegQueue.html) that enables those threads to communicate

Putting these pieces together yields the following introductory [example program](https://github.com/gjf2a/midi_fundsp/blob/master/examples/basic_demo.rs):

```rust
use std::sync::{Arc, Mutex};

use crossbeam_queue::SegQueue;
use crossbeam_utils::atomic::AtomicCell;
use midi_fundsp::{
    io::{get_first_midi_device, start_input_thread, start_output_thread},
    sounds::options,
};
use midir::MidiInput;
use read_input::{shortcut::input, InputBuild};

fn main() -> anyhow::Result<()> {
    let mut midi_in = MidiInput::new("midir reading input")?;
    let in_port = get_first_midi_device(&mut midi_in)?;
    let midi_msgs = Arc::new(SegQueue::new());
    let quit = Arc::new(AtomicCell::new(false));
    
    start_input_thread(midi_msgs.clone(), midi_in, in_port, quit.clone());
    start_output_thread::<10>(midi_msgs, Arc::new(Mutex::new(options())), quit);
    
    input::<String>().msg("Press any key to exit\n").get();
    Ok(())
}
```

The first four lines set up:
* A handle to the first MIDI input device it finds
* A messaging queue to connect the input and output threads
* A flag to instruct the threads to quit

The next two lines call `start_input_thread()` and `start_output_thread()` to 
start the corresponding threads. The table of [fundsp](https://crates.io/crates/fundsp) 
sounds comes from `midi_fundsp::sounds::options()`, but a user can easily assemble their
own custom table of sounds as well.

Once the threads start, the program continues until the user enters a key, handling any
incoming MIDI events as they arrive.

Other [example programs](https://github.com/gjf2a/midi_fundsp/tree/master/examples) show
how to send [different sounds to the left and right stereo channels](https://github.com/gjf2a/midi_fundsp/blob/master/examples/stereo_demo.rs)
and how to [change the selection of synthesizer sound and MIDI input device while running](https://github.com/gjf2a/midi_fundsp/blob/master/examples/choice_demo.rs).

## Notes
* Always compile with `--release`. Sound quality is poor when compiled with `--debug`.
* The following MIDI messages are currently supported:
  * Note On
  * Note Off
  * Pitch Bend
  * Program Change
    * Program change numbers correspond to indices in the `ProgramTable`
    * These can originate either from a MIDI input device or from [software](https://github.com/gjf2a/midi_fundsp/blob/master/examples/choice_demo.rs)
  * All Notes Off
  * All Sound Off
  
