// Since we're handling dice rolls, all integer results will be present after the minimum,
// so probabilities[i] will be the probability to get min_value + i as a final result.
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone)]
pub struct Distribution {
    probabilities: Vec<f64>,
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

    pub fn scale(&mut self, factor: f64) {
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
    /// forcing all the possible permutation and enumerating all of them will be faster
    /// !todo("Write the brute force code and benchmark it against this one, find the sweet spot where the brute force becomes faster, and call the brute force when you know it's faster than the general case")
    pub fn get_sum_highest_k_distribution(&self, number_of_dice: u32, number_of_kept_dice: u32) -> Distribution {
        let number_of_dice_usize = number_of_dice as usize;
        let number_of_kept_dice_usize = number_of_kept_dice as usize;

        // Base: only the smallest face — all dice land on it
        let mut state_below_face = vec![vec![Distribution::constant(0); number_of_kept_dice_usize + 1]; number_of_dice_usize + 1];
        let v0 = self.min_value;
        for cur_number_of_dice in 0..=number_of_dice_usize {
            for cur_number_of_kept_dice in 0..=cur_number_of_dice.min(number_of_kept_dice_usize) {
                state_below_face[cur_number_of_dice][cur_number_of_kept_dice] = Distribution::constant(cur_number_of_kept_dice as u32 * v0);
            }
        }

        let mut prob_lower = self.probabilities[0]; // cumulative probability of faces in state_below_face
        for idx in 1..self.probabilities.len() {
            let cur_face_value = self.min_value + idx as u32;
            let cur_face_prob = self.probabilities[idx];

            let prob_total = prob_lower + cur_face_prob;  // P(lower than or equal to this face)
            let mut state_up_to_face = vec![vec![Distribution::constant(0); number_of_kept_dice_usize + 1]; number_of_dice_usize + 1];
            for cur_number_of_dice in 0..=number_of_dice_usize {
                for cur_number_of_kept_dice in 0..=cur_number_of_dice.min(number_of_kept_dice_usize) {
                    let mut cell_distribution: Option<Distribution> = None;
                    for cur_number_of_dice_landing_on_face in 0..=cur_number_of_dice {
                        let kept = cur_number_of_dice_landing_on_face.min(cur_number_of_kept_dice);
                        let prob_dice_landing_on_face = get_combinations(cur_number_of_dice as u32, cur_number_of_dice_landing_on_face as u32) as f64
                            * (cur_face_prob / prob_total).powi(cur_number_of_dice_landing_on_face as i32)
                            * (prob_lower / prob_total).powi((cur_number_of_dice - cur_number_of_dice_landing_on_face) as i32);
                        let mut branch_distribution = state_below_face[cur_number_of_dice - cur_number_of_dice_landing_on_face][cur_number_of_kept_dice - kept].clone();
                        branch_distribution.shift_by(kept as u32 * cur_face_value);
                        branch_distribution.scale(prob_dice_landing_on_face);
                        match cell_distribution.as_mut() {
                            None => cell_distribution = Some(branch_distribution),
                            Some(d) => d.merge(&branch_distribution),
                        }
                    }
                    state_up_to_face[cur_number_of_dice][cur_number_of_kept_dice] = cell_distribution.expect("at least one dice_on_face in 0..=cur_number_of_dice");
                }
            }
            prob_lower = prob_total;
            state_below_face = state_up_to_face;
        }

        state_below_face[number_of_dice_usize][number_of_kept_dice_usize].clone()
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
        probabilities: vec![1.0 / roll.dice_size as f64; roll.dice_size as usize],
        min_value: 1
    };

    if let Some(modifier) = & roll.reroll_modifier {
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

                    bad_dice_distribution.probabilities.push(1.0 / bad_rolls.len() as f64);

                    if good_dice_distribution.min_value > 0 {
                        good_dice_distribution.probabilities.push(0.0);
                    }
                } else {
                    if good_dice_distribution.min_value == 0 {
                        good_dice_distribution.min_value = item;
                    }

                    good_dice_distribution.probabilities.push(1.0 / (roll.dice_size as usize - bad_rolls.len()) as f64);

                    if bad_dice_distribution.min_value > 0 {
                        bad_dice_distribution.probabilities.push(0.0);
                    }
                }
            })
        ;

        let last_non_zero = bad_dice_distribution.probabilities.iter()
            .enumerate()
            .rev()
            .find(|(pos, item)| **item > 0.0)
            ;

        if let Some((pos, val)) = last_non_zero {
            bad_dice_distribution.probabilities.truncate(pos + 1);
        }

        let last_non_zero = good_dice_distribution.probabilities.iter()
            .enumerate()
            .rev()
            .find(|(pos, item)| **item > 0.0)
            ;

        if let Some((pos, val)) = last_non_zero {
            good_dice_distribution.probabilities.truncate(pos + 1);
        }

        for number_of_bad_dice in 0..=roll.number_of_dice {
            let number_of_good_dice = roll.number_of_dice - number_of_bad_dice;

            let probability_of_scenario = get_combinations(roll.number_of_dice, number_of_bad_dice) as f64
                * probability_of_bad_dice.powi(number_of_bad_dice as i32) as f64
                * probability_of_good_dice.powi(number_of_good_dice as i32) as f64
                ;

            let number_of_rerolled_dice = dice_to_reroll.min(number_of_bad_dice);
            let number_of_kept_bad_dice = number_of_bad_dice.saturating_sub(dice_to_reroll);

            let scenario_distribution =
                match &roll.keeping_result_modifier {
                    Some(KeepingResultModifier::KeepHighest { number_of_dice }) => {
                        compute_scenario_keep_n(
                            *number_of_dice, true,
                            number_of_good_dice, &good_dice_distribution,
                            number_of_bad_dice, number_of_kept_bad_dice,
                            number_of_rerolled_dice, &bad_dice_distribution,
                            &base_dice_distribution,
                        )
                    }
                    Some(KeepingResultModifier::KeepLowest { number_of_dice }) => {
                        compute_scenario_keep_n(
                            *number_of_dice, false,
                            number_of_good_dice, &good_dice_distribution,
                            number_of_bad_dice, number_of_kept_bad_dice,
                            number_of_rerolled_dice, &bad_dice_distribution,
                            &base_dice_distribution,
                        )
                    }
                    None => {
                        compute_scenario_no_keep(
                            number_of_good_dice, &good_dice_distribution,
                            number_of_bad_dice, number_of_kept_bad_dice,
                            number_of_rerolled_dice, &bad_dice_distribution,
                            &base_dice_distribution,
                        )
                    }
                };

            let mut weighted = scenario_distribution;
            weighted.scale(probability_of_scenario);
            if let Some(r) = &mut res {
                r.merge(& weighted);
            } else {
                res = Some(weighted);
            }
        }
    } else {
        match &roll.keeping_result_modifier {
            Some(KeepingResultModifier::KeepHighest { number_of_dice }) => {
                let k = (*number_of_dice).min(roll.number_of_dice);
                res = Some(base_dice_distribution.get_sum_highest_k_distribution(
                    roll.number_of_dice, k,
                ));
            }
            Some(KeepingResultModifier::KeepLowest { number_of_dice }) => {
                let k = (*number_of_dice).min(roll.number_of_dice);
                let rev_base = reverse_distribution(&base_dice_distribution, roll.dice_size);
                let rev_result = rev_base.get_sum_highest_k_distribution(roll.number_of_dice, k);
                let max_val = rev_result.min_value + rev_result.probabilities.len() as u32 - 1;
                let mut probs = rev_result.probabilities.clone();
                probs.reverse();
                res = Some(Distribution {
                    probabilities: probs,
                    min_value: k * (roll.dice_size + 1) - max_val,
                });
            }
            None => {
                let mut dist = Distribution { probabilities: vec![1.0], min_value: 0 };
                for _ in 0..roll.number_of_dice {
                    dist.add(&base_dice_distribution);
                }
                res = Some(dist);
            }
        }
    }

    res.expect("at least one scenario should have been processed")
}

fn compute_scenario_no_keep(
    number_of_good_dice: u32,
    good_dist: &Distribution,
    number_of_bad_dice: u32,
    number_of_kept_bad_dice: u32,
    number_of_rerolled_dice: u32,
    bad_dist: &Distribution,
    base_dist: &Distribution,
) -> Distribution {
    let mut dist = Distribution {
        probabilities: vec![1.0f64],
        min_value: 0,
    };

    if number_of_kept_bad_dice > 0 {
        let kept_bad_sum = bad_dist.get_sum_highest_k_distribution(
            number_of_bad_dice,
            number_of_kept_bad_dice,
        );
        dist.add(&kept_bad_sum);
    }

    for _ in 0..number_of_rerolled_dice {
        dist.add(base_dist);
    }

    for _ in 0..number_of_good_dice {
        dist.add(good_dist);
    }

    dist
}

fn compute_scenario_keep_n(
    k: u32,
    descending: bool,
    number_of_good_dice: u32,
    good_dist: &Distribution,
    number_of_bad_dice: u32,
    number_of_kept_bad_dice: u32,
    number_of_rerolled_dice: u32,
    bad_dist: &Distribution,
    base_dist: &Distribution,
) -> Distribution {
    let k = k.min(number_of_good_dice + number_of_kept_bad_dice + number_of_rerolled_dice) as usize;

    let mut state: HashMap<Vec<u32>, f64> = HashMap::new();
    state.insert(Vec::new(), 1.0);

    fn add_dice_to_state(
        state: HashMap<Vec<u32>, f64>,
        dist: &Distribution,
        count: u32,
        k: usize,
        descending: bool,
    ) -> HashMap<Vec<u32>, f64> {
        let values: Vec<u32> = (dist.min_value..dist.min_value + dist.probabilities.len() as u32).collect();
        let mut current = state;
        for _ in 0..count {
            let mut next: HashMap<Vec<u32>, f64> = HashMap::new();
            for (top_vals, prob) in &current {
                for (vi, &v) in values.iter().enumerate() {
                    let p_die = dist.probabilities[vi];
                    let mut new_vals = top_vals.clone();
                    new_vals.push(v);
                    if descending {
                        new_vals.sort_unstable_by(|a, b| b.cmp(a));
                    } else {
                        new_vals.sort_unstable();
                    }
                    new_vals.truncate(k);
                    *next.entry(new_vals).or_insert(0.0) += prob * p_die;
                }
            }
            current = next;
        }
        current
    }

    let kept_bad_to_model = number_of_kept_bad_dice.min(k as u32);
    if kept_bad_to_model > 0 {
        // Bad dice selection always keeps HIGHEST (original values), regardless of final keep direction
        let mut bad_state: HashMap<Vec<u32>, f64> = HashMap::new();
        bad_state.insert(Vec::new(), 1.0);
        bad_state = add_dice_to_state(bad_state, bad_dist, number_of_bad_dice, kept_bad_to_model as usize, true);

        let mut merged: HashMap<Vec<u32>, f64> = HashMap::new();
        for (main_vals, main_prob) in &state {
            for (bad_vals, bad_prob) in &bad_state {
                let mut combined = main_vals.clone();
                combined.extend(bad_vals.iter().copied());
                if descending {
                    combined.sort_unstable_by(|a, b| b.cmp(a));
                } else {
                    combined.sort_unstable();
                }
                combined.truncate(k);
                *merged.entry(combined).or_insert(0.0) += main_prob * bad_prob;
            }
        }
        state = merged;
    }

    state = add_dice_to_state(state, base_dist, number_of_rerolled_dice, k, descending);
    state = add_dice_to_state(state, good_dist, number_of_good_dice, k, descending);

    let mut sum_probs: HashMap<u32, f64> = HashMap::new();
    for (vals, prob) in &state {
        let sum: u32 = vals.iter().sum();
        *sum_probs.entry(sum).or_insert(0.0) += prob;
    }

    let min_sum = sum_probs.keys().min().copied().unwrap_or(0);
    let max_sum = sum_probs.keys().max().copied().unwrap_or(0);
    let mut probs = vec![0.0f64; (max_sum - min_sum + 1) as usize];
    for (s, p) in sum_probs {
        probs[(s - min_sum) as usize] = p;
    }

    Distribution { probabilities: probs, min_value: min_sum }
}

fn reverse_distribution(dist: &Distribution, dice_size: u32) -> Distribution {
    let max_v = dice_size;
    let original_max = dist.min_value + dist.probabilities.len() as u32 - 1;
    let new_min = max_v + 1 - original_max;
    let mut probs = dist.probabilities.clone();
    probs.reverse();
    Distribution { probabilities: probs, min_value: new_min }
}

fn distribution_of_max(
    n: u32,
    dist: &Distribution,
    max_value: usize,
) -> Distribution {
    let mut cdf = 0.0f64;
    let mut probs = vec![0.0f64; max_value + 1];
    let mut min = u32::MAX;
    for i in 0..dist.probabilities.len() {
        cdf += dist.probabilities[i];
        let prev_cdf = cdf - dist.probabilities[i];
        let v = dist.min_value + i as u32;
        let p_val = cdf.powi(n as i32) - prev_cdf.powi(n as i32);
        if p_val > 0.0 {
            probs[v as usize] = p_val;
            if v < min { min = v; }
        }
    }
    // Compact: remove leading zeros
    let leading_zeros = probs.iter().position(|&p| p > 0.0).unwrap_or(0);
    probs.drain(0..leading_zeros);
    Distribution {
        probabilities: probs,
        min_value: min,
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::max;
    use crate::maths::dice::KeepingResultModifier::KeepHighest;
    use crate::maths::dice::KeepingResultModifier::KeepLowest;
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
                0.004629629629629629,
                0.013888888888888888,
                0.027777777777777776,
                0.046296296296296294,
                0.06944444444444445,
                0.09722222222222224,
                0.11574074074074074,
                0.125,
                0.125,
                0.11574074074074073,
                0.09722222222222222,
                0.06944444444444445,
                0.046296296296296294,
                0.027777777777777776,
                0.013888888888888888,
                0.004629629629629629
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

        let sum: f64 = brute_forced_probabilities.probabilities.iter().sum();

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
            let expected = (2.0 * (i + 1) as f64 - 1.0) / 400.0;
            assert!((result.probabilities[i as usize] - expected).abs() < 1e-6,
                "i={i}: expected {expected}, got {}", result.probabilities[i as usize]);
        }
    }

    #[test]
    fn it_correctly_computes_sum_of_highest_k_with_weirder_distributions() {
        let d = Distribution {
            probabilities: vec![0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.0, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.1],
            min_value: 1
        };

        // brute force of keep highest 2 out of three from distribution d
        let mut brute_forced_probabilities = Distribution {
            probabilities: vec![0.0; d.probabilities.len() * 2 - 1],
            min_value: 2
        };

        let possible_values = vec![1, 2, 3, 4, 5, 6, 7, 8, 10, 20];

        for i in 0..10 {
            for j in 0..10 {
                for k in 0..10 {
                    let min = possible_values[i].min(possible_values[j].min(possible_values[k]));
                    let sum = possible_values[i] + possible_values[j] + possible_values[k] - min;

                    brute_forced_probabilities.probabilities[(sum - brute_forced_probabilities.min_value) as usize] += 1.0;
                }
            }
        }

        let sum: f64 = brute_forced_probabilities.probabilities.iter().sum();

        for i in 0..brute_forced_probabilities.probabilities.len() {
            brute_forced_probabilities.probabilities[i] /= sum;
        }

        let result = d.get_sum_highest_k_distribution(3, 2);

        assert_eq!(result.min_value, 2);
        assert_eq!(result.probabilities.len(), 39);

        for i in 0..result.probabilities.len() {
            assert!((result.probabilities[i] - brute_forced_probabilities.probabilities[i]).abs() < 1e-6,
                "value {}: brute_force {}, result {}",
                result.min_value + i as u32,
                brute_forced_probabilities.probabilities[i],
                result.probabilities[i]);
        }
    }

    #[test]
    fn it_correctly_computes_sum_of_highest_k_with_weirder_non_uniform_distributions() {
        let d = Distribution {
            probabilities: vec![0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.0, 0.0, 0.0, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.2],
            min_value: 1
        };

        // brute force of keep highest 2 out of three from distribution d
        let mut brute_forced_probabilities = Distribution {
            probabilities: vec![0.0; d.probabilities.len() * 2 - 1],
            min_value: 2
        };

        let possible_values = vec![1, 2, 3, 4, 5, 6, 10, 10, 20, 20];

        for i in 0..10 {
            for j in 0..10 {
                for k in 0..10 {
                    let min = possible_values[i].min(possible_values[j].min(possible_values[k]));
                    let sum = possible_values[i] + possible_values[j] + possible_values[k] - min;

                    brute_forced_probabilities.probabilities[(sum - brute_forced_probabilities.min_value) as usize] += 1.0;
                }
            }
        }

        let sum: f64 = brute_forced_probabilities.probabilities.iter().sum();

        for i in 0..brute_forced_probabilities.probabilities.len() {
            brute_forced_probabilities.probabilities[i] /= sum;
        }

        let result = d.get_sum_highest_k_distribution(3, 2);

        assert_eq!(result.min_value, 2);
        assert_eq!(result.probabilities.len(), 39);

        for i in 0..result.probabilities.len() {
            assert!((result.probabilities[i] - brute_forced_probabilities.probabilities[i]).abs() < 1e-6,
                "value {}: brute_force {}, result {}",
                result.min_value + i as u32,
                brute_forced_probabilities.probabilities[i],
                result.probabilities[i]);
        }
    }

    #[test]
    fn it_correctly_computes_sum_of_highest_k_with_reroll_distributions() {
        let d = Distribution {
            probabilities: vec![1.0 / 20.0; 20],
            min_value: 1
        };

        // brute force of keep highest 2 out of three from distribution d with 2 dice to reroll if < 6
        let mut brute_forced_probabilities = Distribution {
            probabilities: vec![0.0; d.probabilities.len() * 2 - 1],
            min_value: 2
        };

        for i in 1..=20 {
            for j in 1..=20 {
                for k in 1..=20 {
                    let mut sorted_values = vec![i, j, k];
                    sorted_values.sort();

                    if sorted_values[0] < 6 {
                        for i in 1..=20 {
                            if sorted_values[1] < 6 {
                                for j in 1..=20 {
                                    let mut sorted_values = vec![i, j, sorted_values[2]];
                                    sorted_values.sort();

                                    brute_forced_probabilities.probabilities[(sorted_values[1] + sorted_values[2] - brute_forced_probabilities.min_value) as usize] += 1.0 / 400.0;
                                }
                            } else {
                                let mut sorted_values = vec![i, sorted_values[1], sorted_values[2]];
                                sorted_values.sort();

                                brute_forced_probabilities.probabilities[(sorted_values[1] + sorted_values[2] - brute_forced_probabilities.min_value) as usize] += 1.0 / 20.0;
                            }
                        }
                    } else {
                        brute_forced_probabilities.probabilities[(sorted_values[1] + sorted_values[2] - brute_forced_probabilities.min_value) as usize] += 1.0;
                    }
                }
            }
        }

        let sum: f64 = brute_forced_probabilities.probabilities.iter().sum();

        for i in 0..brute_forced_probabilities.probabilities.len() {
            brute_forced_probabilities.probabilities[i] /= sum;
        }

        let result = get_dice_roll_distribution(& DiceRoll {
            number_of_dice: 3,
            dice_size: 20,
            reroll_modifier: Some(RerollIfLower {dice_to_reroll: 2, number: 6}),
            clamping_modifier: None,
            keeping_result_modifier: Some(KeepHighest {number_of_dice: 2})
        });

        dbg!(& brute_forced_probabilities, & result);

        for i in 0..result.probabilities.len() {
            assert!((result.probabilities[i] - brute_forced_probabilities.probabilities[i]).abs() < 1e-6,
                "value {}: brute_force {}, result {}",
                result.min_value + i as u32,
                brute_forced_probabilities.probabilities[i],
                result.probabilities[i]);
        }
    }

    #[test]
    fn it_correctly_computes_keep_highest_1_with_reroll_distributions() {
        let mut bf = Distribution {
            probabilities: vec![0.0; 20],  // values 1..=20
            min_value: 1
        };

        for i in 1..=20 {
            for j in 1..=20 {
                for k in 1..=20 {
                    let mut sorted = vec![i, j, k];
                    sorted.sort();

                    if sorted[0] < 6 {
                        for ni in 1..=20 {
                            if sorted[1] < 6 {
                                for nj in 1..=20 {
                                    let mx = *[ni, nj, sorted[2]].iter().max().unwrap();
                                    bf.probabilities[(mx - bf.min_value) as usize] += 1.0 / 400.0;
                                }
                            } else {
                                let mx = *[ni, sorted[1], sorted[2]].iter().max().unwrap();
                                bf.probabilities[(mx - bf.min_value) as usize] += 1.0 / 20.0;
                            }
                        }
                    } else {
                        let mx = sorted[2];
                        bf.probabilities[(mx - bf.min_value) as usize] += 1.0;
                    }
                }
            }
        }

        let sum: f64 = bf.probabilities.iter().sum();
        for p in &mut bf.probabilities { *p /= sum; }

        let result = get_dice_roll_distribution(& DiceRoll {
            number_of_dice: 3, dice_size: 20,
            reroll_modifier: Some(RerollIfLower {dice_to_reroll: 2, number: 6}),
            clamping_modifier: None,
            keeping_result_modifier: Some(KeepHighest {number_of_dice: 1})
        });

        assert_eq!(result.min_value, bf.min_value);
        assert_eq!(result.probabilities.len(), bf.probabilities.len());
        for i in 0..result.probabilities.len() {
            assert!((result.probabilities[i] - bf.probabilities[i]).abs() < 1e-6,
                "value {}: bf {}, result {}", result.min_value + i as u32, bf.probabilities[i], result.probabilities[i]);
        }
    }

    #[test]
    fn it_correctly_computes_keep_highest_3_with_reroll_distributions() {
        // 3d20r2<6kh3 = keep all, should match no-keep for 3 dice
        let mut bf = Distribution {
            probabilities: vec![0.0; 58],  // values 3..=60
            min_value: 3
        };

        for i in 1..=20 {
            for j in 1..=20 {
                for k in 1..=20 {
                    let mut sorted = vec![i, j, k];
                    sorted.sort();

                    if sorted[0] < 6 {
                        for ni in 1..=20 {
                            if sorted[1] < 6 {
                                for nj in 1..=20 {
                                    let sum_val = ni + nj + sorted[2];
                                    bf.probabilities[(sum_val - bf.min_value) as usize] += 1.0 / 400.0;
                                }
                            } else {
                                let sum_val = ni + sorted[1] + sorted[2];
                                bf.probabilities[(sum_val - bf.min_value) as usize] += 1.0 / 20.0;
                            }
                        }
                    } else {
                        let sum_val = sorted[0] + sorted[1] + sorted[2];
                        bf.probabilities[(sum_val - bf.min_value) as usize] += 1.0;
                    }
                }
            }
        }

        let sum: f64 = bf.probabilities.iter().sum();
        for p in &mut bf.probabilities { *p /= sum; }

        let result = get_dice_roll_distribution(& DiceRoll {
            number_of_dice: 3, dice_size: 20,
            reroll_modifier: Some(RerollIfLower {dice_to_reroll: 2, number: 6}),
            clamping_modifier: None,
            keeping_result_modifier: Some(KeepHighest {number_of_dice: 3})
        });

        assert_eq!(result.min_value, bf.min_value);
        assert_eq!(result.probabilities.len(), bf.probabilities.len());
        for i in 0..result.probabilities.len() {
            assert!((result.probabilities[i] - bf.probabilities[i]).abs() < 1e-6,
                "value {}: bf {}, result {}", result.min_value + i as u32, bf.probabilities[i], result.probabilities[i]);
        }
    }

    #[test]
    fn it_correctly_computes_keep_highest_4_with_reroll_distributions() {
        // 3d20r2<6kh4 = keep all (k > dice count), same as kh3
        let mut bf = Distribution {
            probabilities: vec![0.0; 58],
            min_value: 3
        };

        for i in 1..=20 {
            for j in 1..=20 {
                for k in 1..=20 {
                    let mut sorted = vec![i, j, k];
                    sorted.sort();

                    if sorted[0] < 6 {
                        for ni in 1..=20 {
                            if sorted[1] < 6 {
                                for nj in 1..=20 {
                                    let sum_val = ni + nj + sorted[2];
                                    bf.probabilities[(sum_val - bf.min_value) as usize] += 1.0 / 400.0;
                                }
                            } else {
                                let sum_val = ni + sorted[1] + sorted[2];
                                bf.probabilities[(sum_val - bf.min_value) as usize] += 1.0 / 20.0;
                            }
                        }
                    } else {
                        let sum_val = sorted[0] + sorted[1] + sorted[2];
                        bf.probabilities[(sum_val - bf.min_value) as usize] += 1.0;
                    }
                }
            }
        }

        let sum: f64 = bf.probabilities.iter().sum();
        for p in &mut bf.probabilities { *p /= sum; }

        let result = get_dice_roll_distribution(& DiceRoll {
            number_of_dice: 3, dice_size: 20,
            reroll_modifier: Some(RerollIfLower {dice_to_reroll: 2, number: 6}),
            clamping_modifier: None,
            keeping_result_modifier: Some(KeepHighest {number_of_dice: 4})
        });

        assert_eq!(result.min_value, bf.min_value);
        assert_eq!(result.probabilities.len(), bf.probabilities.len());
        for i in 0..result.probabilities.len() {
            assert!((result.probabilities[i] - bf.probabilities[i]).abs() < 1e-6,
                "value {}: bf {}, result {}", result.min_value + i as u32, bf.probabilities[i], result.probabilities[i]);
        }
    }

    #[test]
    fn it_correctly_computes_keep_highest_3_of_5_dice_without_rerolls() {
        // 5d6kh3, brute force 6^5 = 7776 combos
        let mut bf = Distribution {
            probabilities: vec![0.0; 16],  // values 3..=18
            min_value: 3
        };

        for a in 1..=6 {
            for b in 1..=6 {
                for c in 1..=6 {
                    for d in 1..=6 {
                        for e in 1..=6 {
                            let mut vals = vec![a, b, c, d, e];
                            vals.sort_unstable_by(|x, y| y.cmp(x));
                            let sum = vals[0] + vals[1] + vals[2];
                            bf.probabilities[(sum - bf.min_value) as usize] += 1.0;
                        }
                    }
                }
            }
        }

        let total = 6.0f64.powi(5);
        for p in &mut bf.probabilities { *p /= total; }

        let result = get_dice_roll_distribution(& DiceRoll {
            number_of_dice: 5, dice_size: 6,
            reroll_modifier: None,
            clamping_modifier: None,
            keeping_result_modifier: Some(KeepHighest {number_of_dice: 3})
        });

        assert_eq!(result.min_value, bf.min_value, "min_value mismatch");
        assert_eq!(result.probabilities.len(), bf.probabilities.len(), "length mismatch");
        for i in 0..result.probabilities.len() {
            assert!((result.probabilities[i] - bf.probabilities[i]).abs() < 1e-6,
                "value {}: bf {}, result {}", result.min_value + i as u32, bf.probabilities[i], result.probabilities[i]);
        }
    }

    #[test]
    fn it_correctly_computes_keep_highest_3_of_5_dice_with_rerolls() {
        // 5d6r2<3kh3, brute force
        let mut bf = Distribution {
            probabilities: vec![0.0; 16],
            min_value: 3u32
        };

        for a in 1u32..=6 {
            for b in 1u32..=6 {
                for c in 1u32..=6 {
                    for d in 1u32..=6 {
                        for e in 1u32..=6 {
                            let mut sorted = vec![a, b, c, d, e];
                            sorted.sort();
                            let bad_count = sorted.iter().filter(|&&x| x < 3).count();
                            let reroll_count = 2.min(bad_count);
                            let kept_count = bad_count - reroll_count;

                            let good_vals: Vec<u32> = sorted.iter().skip(bad_count).copied().collect();
                            let bad_vals: Vec<u32> = sorted.iter().take(bad_count).copied().collect();

                            if bad_count == 0 {
                                let mut all = good_vals;
                                all.sort_unstable_by(|x, y| y.cmp(x));
                                let sum = all[0] + all[1] + all[2];
                                bf.probabilities[(sum - bf.min_value) as usize] += 1.0;
                            } else if bad_count == 1 {
                                for r1 in 1u32..=6 {
                                    let mut final_vals = vec![r1];
                                    final_vals.extend(&good_vals);
                                    final_vals.sort_unstable_by(|x, y| y.cmp(x));
                                    let sum = final_vals[0] + final_vals[1] + final_vals[2];
                                    bf.probabilities[(sum - bf.min_value) as usize] += 1.0 / 6.0;
                                }
                            } else {
                                for r1 in 1u32..=6 {
                                    for r2 in 1u32..=6 {
                                        let mut final_vals: Vec<u32> = vec![r1, r2];
                                        let kept_bad: Vec<u32> = bad_vals.iter()
                                            .rev().take(kept_count).copied().collect();
                                        final_vals.extend(&kept_bad);
                                        final_vals.extend(&good_vals);
                                        final_vals.sort_unstable_by(|x, y| y.cmp(x));
                                        let sum = final_vals[0] + final_vals[1] + final_vals[2];
                                        bf.probabilities[(sum - bf.min_value) as usize] += 1.0 / 36.0;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let total: f64 = bf.probabilities.iter().sum();
        for p in &mut bf.probabilities { *p /= total; }

        let result = get_dice_roll_distribution(& DiceRoll {
            number_of_dice: 5, dice_size: 6,
            reroll_modifier: Some(RerollIfLower {dice_to_reroll: 2, number: 3}),
            clamping_modifier: None,
            keeping_result_modifier: Some(KeepHighest {number_of_dice: 3})
        });

        assert_eq!(result.min_value, bf.min_value, "min_value mismatch");
        assert_eq!(result.probabilities.len(), bf.probabilities.len(), "length mismatch");
        for i in 0..result.probabilities.len() {
            assert!((result.probabilities[i] - bf.probabilities[i]).abs() < 1e-6,
                "value {}: bf {}, result {}", result.min_value + i as u32, bf.probabilities[i], result.probabilities[i]);
        }
    }

    #[test]
    fn it_correctly_computes_keep_lowest_1_with_reroll_distributions() {
        let mut bf = Distribution { probabilities: vec![0.0; 20], min_value: 1u32 };
        for i in 1u32..=20 { for j in 1u32..=20 { for k in 1u32..=20 {
            let mut sorted = vec![i, j, k]; sorted.sort();
            if sorted[0] < 6 {
                for ni in 1u32..=20 {
                    if sorted[1] < 6 {
                        for nj in 1u32..=20 {
                            let mn = *[ni, nj, sorted[2]].iter().min().unwrap();
                            bf.probabilities[(mn - bf.min_value) as usize] += 1.0 / 400.0;
                        }
                    } else {
                        let mn = *[ni, sorted[1], sorted[2]].iter().min().unwrap();
                        bf.probabilities[(mn - bf.min_value) as usize] += 1.0 / 20.0;
                    }
                }
            } else { bf.probabilities[(sorted[0] - bf.min_value) as usize] += 1.0; }
        }}}
        let sum: f64 = bf.probabilities.iter().sum();
        for p in &mut bf.probabilities { *p /= sum; }
        let result = get_dice_roll_distribution(&DiceRoll {
            number_of_dice: 3, dice_size: 20,
            reroll_modifier: Some(RerollIfLower {dice_to_reroll: 2, number: 6}),
            clamping_modifier: None, keeping_result_modifier: Some(KeepLowest {number_of_dice: 1})
        });
        assert_eq!(result.min_value, bf.min_value);
        assert_eq!(result.probabilities.len(), bf.probabilities.len());
        for i in 0..result.probabilities.len() {
            assert!((result.probabilities[i] - bf.probabilities[i]).abs() < 1e-6,
                "value {}: bf {}, result {}", result.min_value + i as u32, bf.probabilities[i], result.probabilities[i]);
        }
    }

    #[test]
    fn it_correctly_computes_keep_lowest_2_with_reroll_distributions() {
        let mut bf = Distribution { probabilities: vec![0.0; 39], min_value: 2u32 };
        for i in 1u32..=20 { for j in 1u32..=20 { for k in 1u32..=20 {
            let mut sorted = vec![i, j, k]; sorted.sort();
            if sorted[0] < 6 {
                for ni in 1u32..=20 {
                    if sorted[1] < 6 {
                        for nj in 1u32..=20 {
                            let mut vals = vec![ni, nj, sorted[2]]; vals.sort();
                            bf.probabilities[(vals[0] + vals[1] - bf.min_value) as usize] += 1.0 / 400.0;
                        }
                    } else {
                        let mut vals = vec![ni, sorted[1], sorted[2]]; vals.sort();
                        bf.probabilities[(vals[0] + vals[1] - bf.min_value) as usize] += 1.0 / 20.0;
                    }
                }
            } else { bf.probabilities[(sorted[0] + sorted[1] - bf.min_value) as usize] += 1.0; }
        }}}
        let sum: f64 = bf.probabilities.iter().sum();
        for p in &mut bf.probabilities { *p /= sum; }
        let result = get_dice_roll_distribution(&DiceRoll {
            number_of_dice: 3, dice_size: 20,
            reroll_modifier: Some(RerollIfLower {dice_to_reroll: 2, number: 6}),
            clamping_modifier: None, keeping_result_modifier: Some(KeepLowest {number_of_dice: 2})
        });
        assert_eq!(result.min_value, bf.min_value);
        assert_eq!(result.probabilities.len(), bf.probabilities.len());
        for i in 0..result.probabilities.len() {
            assert!((result.probabilities[i] - bf.probabilities[i]).abs() < 1e-6,
                "value {}: bf {}, result {}", result.min_value + i as u32, bf.probabilities[i], result.probabilities[i]);
        }
    }

    #[test]
    fn it_correctly_computes_keep_lowest_3_of_5_dice_without_rerolls() {
        // 5d6kl3
        let mut bf = Distribution { probabilities: vec![0.0; 16], min_value: 3u32 };
        for a in 1u32..=6 { for b in 1u32..=6 { for c in 1u32..=6 { for d in 1u32..=6 { for e in 1u32..=6 {
            let mut vals = vec![a, b, c, d, e];
            vals.sort(); // ascending
            let sum = vals[0] + vals[1] + vals[2];
            bf.probabilities[(sum - bf.min_value) as usize] += 1.0;
        }}}}}
        let total = 6.0f64.powi(5);
        for p in &mut bf.probabilities { *p /= total; }
        let result = get_dice_roll_distribution(&DiceRoll {
            number_of_dice: 5, dice_size: 6, reroll_modifier: None,
            clamping_modifier: None, keeping_result_modifier: Some(KeepLowest {number_of_dice: 3})
        });
        assert_eq!(result.min_value, bf.min_value);
        assert_eq!(result.probabilities.len(), bf.probabilities.len());
        for i in 0..result.probabilities.len() {
            assert!((result.probabilities[i] - bf.probabilities[i]).abs() < 1e-6,
                "value {}: bf {}, result {}", result.min_value + i as u32, bf.probabilities[i], result.probabilities[i]);
        }
    }

    #[test]
    fn it_correctly_computes_keep_lowest_3_of_5_dice_with_rerolls() {
        // 5d6r2<3kl3
        let mut bf = Distribution { probabilities: vec![0.0; 16], min_value: 3u32 };
        for a in 1u32..=6 { for b in 1u32..=6 { for c in 1u32..=6 { for d in 1u32..=6 { for e in 1u32..=6 {
            let mut sorted = vec![a, b, c, d, e]; sorted.sort();
            let bad_count = sorted.iter().filter(|&&x| x < 3).count();
            let reroll_count = 2.min(bad_count);
            let kept_count = bad_count - reroll_count;
            let good_vals: Vec<u32> = sorted.iter().skip(bad_count).copied().collect();
            let bad_vals: Vec<u32> = sorted.iter().take(bad_count).copied().collect();
            if bad_count == 0 {
                let mut all = good_vals; all.sort();
                bf.probabilities[(all[0] + all[1] + all[2] - bf.min_value) as usize] += 1.0;
            } else if bad_count == 1 {
                for r1 in 1u32..=6 {
                    let mut final_vals = vec![r1]; final_vals.extend(&good_vals); final_vals.sort();
                    bf.probabilities[(final_vals[0] + final_vals[1] + final_vals[2] - bf.min_value) as usize] += 1.0 / 6.0;
                }
            } else {
                for r1 in 1u32..=6 { for r2 in 1u32..=6 {
                    let mut final_vals: Vec<u32> = vec![r1, r2];
                    let kept_bad: Vec<u32> = bad_vals.iter().rev().take(kept_count).copied().collect();
                    final_vals.extend(&kept_bad);
                    final_vals.extend(&good_vals);
                    final_vals.sort();
                    bf.probabilities[(final_vals[0] + final_vals[1] + final_vals[2] - bf.min_value) as usize] += 1.0 / 36.0;
                }}
            }
        }}}}}
        let total: f64 = bf.probabilities.iter().sum();
        for p in &mut bf.probabilities { *p /= total; }
        let result = get_dice_roll_distribution(&DiceRoll {
            number_of_dice: 5, dice_size: 6,
            reroll_modifier: Some(RerollIfLower {dice_to_reroll: 2, number: 3}),
            clamping_modifier: None, keeping_result_modifier: Some(KeepLowest {number_of_dice: 3})
        });
        assert_eq!(result.min_value, bf.min_value);
        assert_eq!(result.probabilities.len(), bf.probabilities.len());
        for i in 0..result.probabilities.len() {
            assert!((result.probabilities[i] - bf.probabilities[i]).abs() < 1e-6,
                "value {}: bf {}, result {}", result.min_value + i as u32, bf.probabilities[i], result.probabilities[i]);
        }
    }
}