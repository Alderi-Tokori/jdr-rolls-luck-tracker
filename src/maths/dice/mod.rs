// Since we're handling dice rolls, all integer results will be present after the minimum,
// so probabilities[i] will be the probability to get min_value + i as a final result.
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
                    Some(KeepingResultModifier::KeepHighest { number_of_dice: 2 }) => {
                        compute_scenario_keep_highest_2(
                            number_of_good_dice, &good_dice_distribution,
                            number_of_bad_dice, number_of_kept_bad_dice,
                            number_of_rerolled_dice, &bad_dice_distribution,
                            &base_dice_distribution,
                        )
                    }
                    Some(_) => {
                        unimplemented!("only KeepHighest(2) is supported for now")
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

fn compute_scenario_keep_highest_2(
    number_of_good_dice: u32,
    good_dist: &Distribution,
    number_of_bad_dice: u32,
    number_of_kept_bad_dice: u32,
    number_of_rerolled_dice: u32,
    bad_dist: &Distribution,
    base_dist: &Distribution,
) -> Distribution {
    let max_value_good = (good_dist.min_value + good_dist.probabilities.len() as u32 - 1) as usize;
    let max_value_bad = (bad_dist.min_value + bad_dist.probabilities.len() as u32 - 1) as usize;
    let max_value_base = (base_dist.min_value + base_dist.probabilities.len() as u32 - 1) as usize;
    let max_value = max_value_good.max(max_value_bad).max(max_value_base);

    let mut state = vec![vec![0.0f64; max_value + 1]; max_value + 1];
    state[0][0] = 1.0;

    if number_of_kept_bad_dice == 1 {
        let dist = distribution_of_max(
            number_of_bad_dice,
            bad_dist,
            max_value,
        );
        state = add_single_die_to_top2_state(&state, &dist, max_value);
    } else if number_of_kept_bad_dice > 1 {
        unimplemented!("kept_bad > 1 not yet supported analytically");
    }

    if number_of_rerolled_dice == 1 {
        state = add_single_die_to_top2_state(&state, base_dist, max_value);
    } else if number_of_rerolled_dice >= 2 {
        state = add_iid_group_to_top2_state(&state, base_dist, number_of_rerolled_dice, max_value);
    }

    if number_of_good_dice == 1 {
        state = add_single_die_to_top2_state(&state, good_dist, max_value);
    } else if number_of_good_dice >= 2 {
        state = add_iid_group_to_top2_state(&state, good_dist, number_of_good_dice, max_value);
    }

    top2_state_to_distribution(&state)
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

fn add_single_die_to_top2_state(
    state: &[Vec<f64>],
    die_dist: &Distribution,
    max_value: usize,
) -> Vec<Vec<f64>> {
    let mut new_state = vec![vec![0.0f64; max_value + 1]; max_value + 1];

    let values: Vec<u32> = (die_dist.min_value..die_dist.min_value + die_dist.probabilities.len() as u32)
        .filter(|&v| (v as usize) <= max_value)
        .collect();

    for top1 in 0..=max_value {
        for top2 in 0..=top1 {
            let p = state[top1][top2];
            if p == 0.0 {
                continue;
            }
            for (vi, &v) in values.iter().enumerate() {
                let prob_v = die_dist.probabilities[vi];
                let (nt1, nt2) = if v as usize >= top1 {
                    (v as usize, top1)
                } else if v as usize >= top2 {
                    (top1, v as usize)
                } else {
                    (top1, top2)
                };
                new_state[nt1][nt2] += p * prob_v;
            }
        }
    }
    new_state
}

fn add_iid_group_to_top2_state(
    state: &[Vec<f64>],
    die_dist: &Distribution,
    count: u32,
    max_value: usize,
) -> Vec<Vec<f64>> {
    let n = count as usize;
    let values: Vec<u32> = (die_dist.min_value..die_dist.min_value + die_dist.probabilities.len() as u32)
        .filter(|&v| (v as usize) <= max_value)
        .collect();

    let mut cdf = vec![0.0f64];
    for &p in &die_dist.probabilities {
        let last = *cdf.last().unwrap();
        cdf.push(last + p);
    }

    // joint[M][S] = P(max=M, second_max=S), S < M; joint[M][M] for S == M
    let mut joint = vec![vec![0.0f64; max_value + 1]; max_value + 1];

    if n >= 2 {
        for mi in 0..values.len() {
            let m = values[mi] as usize;
            let pm = die_dist.probabilities[mi];
            let fm = cdf[mi + 1];
            let fm1 = cdf[mi];

            joint[m][m] = fm.powi(n as i32)
                - fm1.powi(n as i32)
                - n as f64 * pm * fm1.powi((n - 1) as i32);

            for si in 0..mi {
                let s = values[si] as usize;
                let fs = cdf[si + 1];
                let fs1 = cdf[si];
                joint[m][s] = n as f64 * pm
                    * (fs.powi((n - 1) as i32) - fs1.powi((n - 1) as i32));
            }
        }
    } else {
        for mi in 0..values.len() {
            let m = values[mi] as usize;
            joint[m][0] = die_dist.probabilities[mi];
        }
    }

    let mut new_state = vec![vec![0.0f64; max_value + 1]; max_value + 1];
    for top1 in 0..=max_value {
        for top2 in 0..=top1 {
            let p = state[top1][top2];
            if p == 0.0 {
                continue;
            }
            for m in 0..=max_value {
                for s in 0..=m {
                    let p_ms = joint[m][s];
                    if p_ms == 0.0 {
                        continue;
                    }
                    let mut sorted = [top1, top2, m, s];
                    sorted.sort_unstable_by(|a, b| b.cmp(a));
                    new_state[sorted[0]][sorted[1]] += p * p_ms;
                }
            }
        }
    }
    new_state
}

fn top2_state_to_distribution(state: &[Vec<f64>]) -> Distribution {
    let max_value = state.len() - 1;
    let max_sum = max_value * 2;
    let mut probs = vec![0.0f64; max_sum + 1];

    for top1 in 0..=max_value {
        for top2 in 0..=top1 {
            let p = state[top1][top2];
            if p == 0.0 {
                continue;
            }
            let sum = if top2 > 0 { top1 + top2 } else { top1 };
            probs[sum] += p;
        }
    }

    let leading_zeros = probs.iter().position(|&p| p > 0.0).unwrap_or(0);
    probs.drain(0..leading_zeros);

    Distribution {
        probabilities: probs,
        min_value: leading_zeros as u32,
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::max;
    use crate::maths::dice::KeepingResultModifier::KeepHighest;
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
}