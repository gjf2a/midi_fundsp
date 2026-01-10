use midi_fundsp::{SoundTestResult, sounds::options};

fn main() {
    for (name, func) in options() {
        println!("Testing {name}");
        let result = SoundTestResult::test(func);
        result.report();
    }
}
