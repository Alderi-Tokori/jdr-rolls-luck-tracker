use regex::Regex;

mod tests;

// Since we're handling dice rolls, all integer results will be present after the minimum,
// so probabilities[i] will be the probability to get min_value + i as a final result.
use std::collections::HashMap;
use serde::Serialize;
use crate::maths::dice::RerollModifier::{RerollIfEqual, RerollIfGreater, RerollIfLower};

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Distribution {
    probabilities: Vec<f64>,
    min_value: u32,
}

#[derive(Debug)]
pub enum RerollModifier {
    RerollIfLower { dice_to_reroll: u32, number: u32 },
    RerollIfGreater { dice_to_reroll: u32, number: u32 },
    RerollIfEqual { dice_to_reroll: u32, numbers: Vec<u32> },
}

#[derive(Debug)]
pub enum ClampingModifier {
    Minimum { number: u32 },
    Maximum { number: u32 },
}

#[derive(Debug)]
pub enum KeepingResultModifier {
    KeepHighest { number_of_dice: u32 },
    KeepLowest { number_of_dice: u32 },
}

#[derive(Debug)]
pub struct DiceRoll {
    number_of_dice: u32,
    dice_size: u32,
    reroll_modifier: Option<RerollModifier>,
    clamping_modifier: Option<ClampingModifier>,
    keeping_result_modifier: Option<KeepingResultModifier>,
}

impl DiceRoll {
    pub fn parse(s: &str) -> Option<DiceRoll> {
        let mut res = DiceRoll {
            number_of_dice: 0,
            dice_size: 1,
            reroll_modifier: None,
            clamping_modifier: None,
            keeping_result_modifier: None
        };

        // Basic dice match
        let re = Regex::new(r"^\s*(?<number_of_dice>\d+)d(?<dice_size>\d+)").unwrap();
        if let Some(captures) = re.captures(s) {
            res.number_of_dice = captures["number_of_dice"].parse::<u32>().unwrap();
            res.dice_size = captures["dice_size"].parse::<u32>().unwrap();
        } else {
            return None;
        };

        // Reroll modifiers
        let re = Regex::new(r"r(?<dice_to_reroll>\d+)(?<operation>[><=])(?:(?<number>\d+)|\{(?<numbers>(?:\d+,?)+)})").unwrap();
        if let Some(captures) = re.captures(s) {
            let numbers = match captures.get(3) {
                Some(m) => vec![m.as_str().parse::<u32>().unwrap()],
                None => captures.get(4).unwrap().as_str().split(',').collect::<Vec<_>>()
                    .iter()
                    .map(|e| (**e).parse::<u32>().unwrap())
                    .collect(),
            };

            res.reroll_modifier = match & captures["operation"] {
                "<" => Some(RerollIfLower {
                    dice_to_reroll: captures["dice_to_reroll"].parse::<u32>().unwrap(),
                    number: *numbers.iter().max().unwrap()
                }),
                ">" => Some(RerollIfGreater {
                    dice_to_reroll: captures["dice_to_reroll"].parse::<u32>().unwrap(),
                    number: *numbers.iter().min().unwrap()
                }),
                "=" => Some(RerollIfEqual {
                    dice_to_reroll: captures["dice_to_reroll"].parse::<u32>().unwrap(),
                    numbers
                }),
                _ => None
            };
        }

        // Keeping results modifiers
        let re = Regex::new(r"k(?<keep_type>[hl])(?<number_of_dice>\d+)").unwrap();
        if let Some(captures) = re.captures(s) {
            res.keeping_result_modifier = match & captures["keep_type"] {
                "h" => Some(KeepingResultModifier::KeepHighest {
                    number_of_dice: captures["number_of_dice"].parse::<u32>().unwrap()
                }),
                "l" => Some(KeepingResultModifier::KeepLowest {
                    number_of_dice: captures["number_of_dice"].parse::<u32>().unwrap()
                }),
                _ => None
            };
        }

        // Clamping modifiers
        let re = Regex::new(r"(?<clamp_type>min|max)(?<number>\d+)").unwrap();
        if let Some(captures) = re.captures(s) {
            res.clamping_modifier = match & captures["clamp_type"] {
                "min" => Some(ClampingModifier::Minimum {
                    number: captures["number"].parse::<u32>().unwrap()
                }),
                "max" => Some(ClampingModifier::Maximum {
                    number: captures["number"].parse::<u32>().unwrap()
                }),
                _ => None
            };
        }

        Some(res)
    }
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

    pub fn clamped(&self, modifier: &ClampingModifier) -> Distribution {
        match modifier {
            ClampingModifier::Minimum { number } => {
                let n = *number;
                if n <= self.min_value { return self.clone(); }
                let idx = (n - self.min_value) as usize;
                if idx >= self.probabilities.len() { return Distribution::constant(n); }
                let mut probs = self.probabilities.clone();
                let clamped: f64 = probs[..idx].iter().sum();
                probs.drain(0..idx);
                probs[0] += clamped;
                Distribution { probabilities: probs, min_value: n }
            }
            ClampingModifier::Maximum { number } => {
                let n = *number;
                let max_val = self.min_value + self.probabilities.len() as u32 - 1;
                if n >= max_val { return self.clone(); }
                let idx = (n - self.min_value) as usize;
                let mut probs = self.probabilities.clone();
                let clamped: f64 = probs[idx + 1..].iter().sum();
                probs.truncate(idx + 1);
                probs[idx] += clamped;
                Distribution { probabilities: probs, min_value: self.min_value }
            }
        }
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
                            roll.clamping_modifier.as_ref(),
                        )
                    }
                    Some(KeepingResultModifier::KeepLowest { number_of_dice }) => {
                        compute_scenario_keep_n(
                            *number_of_dice, false,
                            number_of_good_dice, &good_dice_distribution,
                            number_of_bad_dice, number_of_kept_bad_dice,
                            number_of_rerolled_dice, &bad_dice_distribution,
                            &base_dice_distribution,
                            roll.clamping_modifier.as_ref(),
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
        let effective_base = match &roll.clamping_modifier {
            Some(clamp) => base_dice_distribution.clamped(clamp),
            None => base_dice_distribution,
        };
        match &roll.keeping_result_modifier {
            Some(KeepingResultModifier::KeepHighest { number_of_dice }) => {
                let k = (*number_of_dice).min(roll.number_of_dice);
                res = Some(effective_base.get_sum_highest_k_distribution(
                    roll.number_of_dice, k,
                ));
            }
            Some(KeepingResultModifier::KeepLowest { number_of_dice }) => {
                let k = (*number_of_dice).min(roll.number_of_dice);
                let rev_base = reverse_distribution(&effective_base, roll.dice_size);
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
                    dist.add(&effective_base);
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
    clamp: Option<&ClampingModifier>,
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

    // Apply per-die clamping
    if let Some(clamp) = clamp {
        let mut clamped_state: HashMap<Vec<u32>, f64> = HashMap::new();
        for (vals, prob) in &state {
            let clamped_vals: Vec<u32> = vals.iter().map(|&v| match clamp {
                ClampingModifier::Minimum { number } => v.max(*number),
                ClampingModifier::Maximum { number } => v.min(*number),
            }).collect();
            *clamped_state.entry(clamped_vals).or_insert(0.0) += prob;
        }
        state = clamped_state;
    }

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
