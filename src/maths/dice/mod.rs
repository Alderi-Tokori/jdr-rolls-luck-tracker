// Since we're handling dice rolls, all integer results will be present after the minimum,
// so probabilities[i] will be the probability to get min_value + i as a final result.
#[derive(Debug, PartialEq, Clone)]
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
    pub fn constant(c: u32) -> Distribution {
        Distribution {
            probabilities: vec![1.0],
            min_value: c
        }
    }

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

    pub fn shift_by(&mut self, n: u32) {
        self.min_value += n;
    }

    pub fn scale(&mut self, factor: f32) {
        for p in &mut self.probabilities {
            *p *= factor;
        }
    }

    pub fn merge(&mut self, other: & Distribution) {
        let new_min = self.min_value.min(other.min_value);

        let self_offset = (self.min_value - new_min) as usize;
        let other_offset = (other.min_value - new_min) as usize;

        let new_len = (self_offset + self.probabilities.len())
            .max(other_offset + other.probabilities.len());

        let mut new_probs = vec![0.0; new_len];
        for (i, p) in self.probabilities.iter().enumerate() {
            new_probs[self_offset + i] = *p;
        }
        for (i, p) in other.probabilities.iter().enumerate() {
            new_probs[other_offset + i] += *p;
        }

        self.probabilities = new_probs;
        self.min_value = new_min;
    }

    /// Get the sum of the k highest dice among t dice; This is the optimized code for the general case
    /// but for small cases which will be most of the case encountered in a TTRPG setting, brute
    /// forcing all the possible permutation and enumerating all o them will be faster
    /// !todo("Write the brute force code and benchmark it against this one, find the sweet spot where the brute force becomes faster, and call the brute force when you know it's faster than the general case)
    pub fn get_sum_highest_k_distribution(&self, number_of_dice: u32, number_of_kept_dice: u32) -> Distribution {
        let k = number_of_dice as usize;
        let t = number_of_kept_dice as usize;

        // Base: only the smallest face — all dice land on it
        let mut prev = vec![vec![Distribution::constant(0); t + 1]; k + 1];
        let v0 = self.min_value;
        for ki in 0..=k {
            for ti in 0..=ki.min(t) {
                prev[ki][ti] = Distribution::constant(ti as u32 * v0);
            }
        }

        let mut prob_lower = self.probabilities[0]; // cumulative probability of faces already in `prev`
        for idx in 1..self.probabilities.len() {
            let v = self.min_value + idx as u32;
            let p = self.probabilities[idx];

            let total = prob_lower + p; // P(≤ this face)
            let mut curr = vec![vec![Distribution::constant(0); t + 1]; k + 1];
            for ki in 0..=k {
                for ti in 0..=ki.min(t) {
                    let mut dist: Option<Distribution> = None;
                    for c in 0..=ki {
                        let kept = c.min(ti);
                        let prob_c = get_combinations(ki as u32, c as u32) as f32
                            * (p / total).powi(c as i32)
                            * (prob_lower / total).powi((ki - c) as i32);
                        let mut sub = prev[ki - c][ti - kept].clone();
                        sub.shift_by(kept as u32 * v);
                        sub.scale(prob_c);
                        match dist.as_mut() {
                            None => dist = Some(sub),
                            Some(d) => d.merge(&sub),
                        }
                    }
                    curr[ki][ti] = dist.expect("at least one c in 0..=ki");
                }
            }
            prob_lower = total;
            prev = curr;
        }

        prev[k][t].clone()
    }
}

pub fn get_combinations(n: u32, k: u32) -> u64 {
    let k = k.min(n - k);
    let mut result = 1.0;
    for i in 1..=k {
        result *= (n - k + i) as f64;
        result /= i as f64;
    }
    result as u64
}

pub fn get_dice_roll_distribution(roll: & DiceRoll) -> Distribution {
    let mut res: Option<Distribution> = None;

    let base_dice_distribution = Distribution {
        probabilities: vec![1.0 / roll.dice_size as f32; roll.dice_size as usize],
        min_value: 1
    };

    match &roll.reroll_modifier {
        Some(modifier) => {
            let (dice_to_reroll, bad_rolls) = match modifier {
                RerollModifier::RerollIfLower {dice_to_reroll, number} => (*dice_to_reroll, &(1..*number).collect()),
                RerollModifier::RerollIfGreater {dice_to_reroll, number} => (*dice_to_reroll, &(number+1..=roll.dice_size).collect()),
                RerollModifier::RerollIfEqual {dice_to_reroll, numbers} => (*dice_to_reroll, numbers)
            };

            let probability_of_bad_dice = bad_rolls.len() as f32 / roll.dice_size as f32;
            let probability_of_good_dice = 1.0 - probability_of_bad_dice;

            let mut good_dice_distribution = Distribution {
                probabilities: Vec::with_capacity(roll.dice_size as usize),
                min_value: 0,
            };
            let mut bad_dice_distribution = Distribution {
                probabilities: Vec::with_capacity(roll.dice_size as usize),
                min_value: 0,
            };

            (1..=roll.dice_size)
                .for_each(|item| {
                    if bad_rolls.contains(& item) {
                        if bad_dice_distribution.min_value == 0 {
                            bad_dice_distribution.min_value = item;
                        }

                        bad_dice_distribution.probabilities.push(1.0 / bad_rolls.len() as f32);

                        if good_dice_distribution.min_value > 0 {
                            good_dice_distribution.probabilities.push(0.0);
                        }
                    } else {
                        if good_dice_distribution.min_value == 0 {
                            good_dice_distribution.min_value = item;
                        }

                        good_dice_distribution.probabilities.push(1.0 / (roll.dice_size as usize - bad_rolls.len()) as f32);

                        if bad_dice_distribution.min_value > 0 {
                            bad_dice_distribution.probabilities.push(0.0);
                        }
                    }
                })
            ;

            for number_of_bad_dice in 0..=roll.number_of_dice {
                let number_of_good_dice = roll.number_of_dice - number_of_bad_dice;

                let probability_of_scenario = get_combinations(roll.number_of_dice, number_of_bad_dice) as f32
                    * probability_of_bad_dice.powi(number_of_bad_dice as i32)
                    * probability_of_good_dice.powi(number_of_good_dice as i32)
                ;

                let mut scenario_distribution = Distribution {
                    probabilities: Vec::with_capacity((roll.number_of_dice * roll.dice_size - roll.number_of_dice + 1) as usize),
                    min_value: 0
                };

                scenario_distribution.probabilities.push(1.0);

                if number_of_bad_dice > dice_to_reroll {
                    let bad_dice_to_keep = number_of_bad_dice - dice_to_reroll;

                    let kept_bad_dice_distribution =
                        bad_dice_distribution.get_sum_highest_k_distribution(number_of_bad_dice, bad_dice_to_keep);
                    ;

                    scenario_distribution.add(& kept_bad_dice_distribution);
                }

                let number_of_rerolled_dice = dice_to_reroll.min(number_of_bad_dice);

                for _ in 0..number_of_rerolled_dice {
                    scenario_distribution.add(& base_dice_distribution);
                }

                for _ in 0..number_of_good_dice {
                    scenario_distribution.add(& good_dice_distribution);
                }

                scenario_distribution.scale(probability_of_scenario);
                if let Some(r) = &mut res {
                    r.merge(& scenario_distribution);
                } else {
                    res = Some(scenario_distribution);
                }
            }
        },
        None => ()
    }

    res.expect("at least one scenario should have been processed")
}

#[cfg(test)]
mod tests {
    use std::cmp::max;
    use crate::maths::dice::RerollModifier::RerollIfLower;
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
                    brute_forced_probabilities.probabilities[i + j - brute_forced_probabilities.min_value as usize] += 20.0
                }
            }
        }

        let sum: f32 = brute_forced_probabilities.probabilities.iter().sum();

        for i in 0..brute_forced_probabilities.probabilities.len() {
            brute_forced_probabilities.probabilities[i] /= sum;
        }

        let result = get_dice_roll_distribution(& DiceRoll {
            number_of_dice: 2,
            dice_size: 20,
            reroll_modifier: Some(RerollIfLower {
                dice_to_reroll: 1,
                number: 6
            }),
            clamping_modifier: None,
            keeping_result_modifier: None,
        });

        assert_eq!(result.min_value, brute_forced_probabilities.min_value);
        assert_eq!(result.probabilities.len(), brute_forced_probabilities.probabilities.len());
        for i in 0..result.probabilities.len() {
            assert!((result.probabilities[i] - brute_forced_probabilities.probabilities[i]).abs() < 1e-6,
                "value {}: brute_force {}, result {}",
                result.min_value + i as u32,
                brute_forced_probabilities.probabilities[i],
                result.probabilities[i]);
        }
    }

    #[test]
    fn it_correctly_computes_sum_of_highest_k() {
        let d = Distribution {
            probabilities: vec![1.0 / 20.0; 20],
            min_value: 1
        };

        let result = d.get_sum_highest_k_distribution(2, 1);
        assert_eq!(result.min_value, 1);
        assert_eq!(result.probabilities.len(), 20);
        // P(max=i) = (i^2 - (i-1)^2) / 400 = (2i-1)/400
        for i in 0u32..20 {
            let expected = (2.0 * (i + 1) as f32 - 1.0) / 400.0;
            assert!((result.probabilities[i as usize] - expected).abs() < 1e-6,
                "i={i}: expected {expected}, got {}", result.probabilities[i as usize]);
        }
    }
}