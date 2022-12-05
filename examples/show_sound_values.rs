use midi_fundsp::{sounds::options, SoundTestResult};

fn main() {
    for (name, func) in options() {
        println!("Testing {name}");
        let result = SoundTestResult::test(func);
        result.report();
    }
}
