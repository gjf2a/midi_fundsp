use fundsp::math::midi_hz;

pub fn just_intonation<const M: u8, const P: u16>(midi_pitch: f32) -> f32 {
    let midi_pitch = midi_pitch as u8;

    todo!("Write the rest of this")
}

const WELL_C_MINUS_1: f32 = 8.20354352009375;

/// Derived from: https://www.historicaltuning.com/Chapter8.pdf, Table 8.3
/// This is believed by the author to be Bach's well-temperament.
pub fn well_temperament(midi_pitch: f32) -> f32 {
    let midi_pitch = midi_pitch as u8;
    let octave = (midi_pitch / 12) as f32;
    2.0_f32.powf(octave) * WELL_C_MINUS_1 * match midi_pitch % 12 {
        0 => 1.0,
        1 => 1.058267369,
        2 => 1.119929822,
        3 => 1.187864957,
        4 => 1.254242807,
        5 => 1.3363480780010195,
        6 => 1.411023157998401,
        7 => 1.4966160640051305,
        8 => 1.5856094859970158,
        9 => 1.6761049619985275,
        10 => 1.7797864719968166,
        11 => 1.8813642110048348,
        _ => panic!("Unreachable")
    }
}

#[cfg(test)]
mod tests {
    use crate::tunings::well_temperament;
    use float_eq::assert_float_eq;

    #[test]
    fn test_well() {
        // Corresponds to Table 8.3 in https://www.historicaltuning.com/Chapter8.pdf
        for (midi, hz) in [
            (53.0, 175.404633854),
            (54.0, 185.206238152),
            (55.0, 196.440880223),
            (56.0, 208.121862788),
            (57.0, 220.000000000),
            (58.0, 233.608892472),
            (59.0, 246.941650914),
            (60.0, 262.513392643),
            (61.0, 277.809357360),
            (62.0, 293.996156549),
            (63.0, 311.830459864),
            (64.0, 329.255534464),
        ] {
            assert_float_eq!(well_temperament(midi), hz, abs <= 1e-3);
        }
    }
}