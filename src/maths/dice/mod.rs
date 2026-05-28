// Since we're handling dice rolls, all integer results will be present after the minimum,
// so probabilities[i] will be the probability to get min_value + i as a final result.
#[derive(Debug, PartialEq)]
pub struct Distribution {
    probabilities: Vec<f32>,
    min_value: u32,
}

pub enum RerollModifier {
    RerollIfLower { dice_to_reroll: u32, number: u32 },
    RerollIfGreater { dice_to_reroll: u32, number: u32 },
    RerollIfEqual { dice_to_reroll: u32, numbers: Vec<u32> },
}

pub enum ClampingModifier {
    Minimum { number: u32 },
    Maximum { number: u32 },
}

pub enum KeepingResultModifier {
    KeepHighest { number_of_dice: u32 },
    KeepLowest { number_of_dice: u32 },
}

pub struct DiceRoll {
    number_of_dice: u32,
    dice_size: u32,
    reroll_modifier: Option<RerollModifier>,
    clamping_modifier: Option<ClampingModifier>,
    keeping_result_modifier: Option<KeepingResultModifier>,
}

impl Distribution {
    pub fn add(&mut self, other: & Distribution) {
        let cur_len = self.probabilities.len();

        self.probabilities.resize(cur_len + other.probabilities.len() - 1, 0.0);

        self.min_value += other.min_value;

        for left_hand_side_idx in (0..cur_len).rev() {
            let left_hand_side = self.probabilities[left_hand_side_idx];

            // With very big dice rolls like 100d6, we can get to a point were the outer values are
            // so small they get zero'd; we don't waste any more calculations on those
            if left_hand_side == 0.0 {
                continue;
            }

            for right_hand_side_idx in (0..other.probabilities.len()).rev() {
                let cross_probability = left_hand_side * other.probabilities[right_hand_side_idx];
                if right_hand_side_idx > 0 {
                    self.probabilities[left_hand_side_idx + right_hand_side_idx] += cross_probability;
                } else {
                    self.probabilities[left_hand_side_idx + right_hand_side_idx] = cross_probability;
                }
            }
        }
    }
}

pub fn get_dice_roll_distribution(roll: & DiceRoll) -> Distribution {
    let mut res = Distribution {
        probabilities: Vec::with_capacity((roll.number_of_dice * roll.dice_size - roll.number_of_dice + 1) as usize),
        min_value: 0
    };

    res.probabilities.push(1.0);

    let base_dice_distribution = Distribution {
        probabilities: vec![1.0 / roll.dice_size as f32; roll.dice_size as usize],
        min_value: 1
    };

    res
}

#[cfg(test)]
mod tests {
    use std::cmp::max;
    use super::*;

    #[test]
    fn it_correctly_adds_distributions() {
        let mut d1 = Distribution {
            probabilities: vec![1.0 / 6.0; 6],
            min_value: 1
        };

        let d2 = Distribution {
            probabilities: vec![1.0 / 6.0; 6],
            min_value: 1
        };

        d1.add(& d2);
        d1.add(& d2);

        assert_eq!(d1, Distribution {
            probabilities: vec![
                0.00462963,
                0.01388889,
                0.027777782,
                0.046296302,
                0.06944445,
                0.097222224,
                0.115740746,
                0.12500001,
                0.12500001,
                0.115740746,
                0.097222224,
                0.06944445,
                0.0462963,
                0.02777778,
                0.01388889,
                0.00462963,
            ],
            min_value: 3,
        });
    }

    #[test]
    fn it_correctly_computes_reroll_modifiers() {
        // Manually computing the probabilities for 2d20r1<6
        let mut brute_forced_probabilities = Distribution {
            probabilities: vec![0.0; 39],
            min_value: 2
        };

        for i in 1..=20 {
            for j in 1..=20 {
                if i < 6 || j < 6 {
                    for k in 1..=20 {
                        brute_forced_probabilities.probabilities[max(i, j) + k - brute_forced_probabilities.min_value as usize] += 1.0;
                    }
                } else {
                    brute_forced_probabilities.probabilities[i + j - brute_forced_probabilities.min_value as usize] += 1.0
                }
            }
        }

        let sum: f32 = brute_forced_probabilities.probabilities.iter().sum();

        dbg!(& sum, & brute_forced_probabilities);

        for i in 0..brute_forced_probabilities.probabilities.len() {
            brute_forced_probabilities.probabilities[i] /= sum;
        }

        dbg!(& brute_forced_probabilities);
    }
}