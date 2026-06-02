
#[cfg(test)]
use std::cmp::max;
use crate::maths::dice::KeepingResultModifier::KeepHighest;
use crate::maths::dice::KeepingResultModifier::KeepLowest;
use crate::maths::dice::RerollModifier::RerollIfLower;
use crate::maths::dice::RerollModifier::RerollIfGreater;
use crate::maths::dice::RerollModifier::RerollIfEqual;
use crate::maths::dice::ClampingModifier;
use crate::maths::dice::ClampingModifier::{Maximum, Minimum};
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
fn it_correctly_computes_sum_of_highest_2_with_reroll_distributions() {
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

#[test]
fn it_correctly_computes_reroll_greater_with_keep_lowest() {
    // 5d6r2>3kl3: reroll up to 2 if > 3 (bad=4,5,6), keep lowest 3
    let mut bf = Distribution { probabilities: vec![0.0; 16], min_value: 3u32 };
    for a in 1u32..=6 { for b in 1u32..=6 { for c in 1u32..=6 { for d in 1u32..=6 { for e in 1u32..=6 {
        let mut sorted = vec![a, b, c, d, e]; sorted.sort();
        let bad_count = sorted.iter().filter(|&&x| x > 3).count();
        let reroll_count = 2.min(bad_count);
        let kept_count = bad_count - reroll_count;
        let good_vals: Vec<u32> = sorted.iter().take(5 - bad_count).copied().collect();
        let bad_vals: Vec<u32> = sorted.iter().skip(5 - bad_count).copied().collect();
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
        reroll_modifier: Some(RerollIfGreater {dice_to_reroll: 2, number: 3}),
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
fn it_correctly_computes_reroll_equal_keep_all() {
    // 5d6r2={2,5}: reroll up to 2 if equal to 2 or 5, keep all 5
    let mut bf = Distribution { probabilities: vec![0.0; 26], min_value: 5u32 };
    for a in 1u32..=6 { for b in 1u32..=6 { for c in 1u32..=6 { for d in 1u32..=6 { for e in 1u32..=6 {
        let mut sorted = vec![a, b, c, d, e]; sorted.sort();
        let bad_count = sorted.iter().filter(|&&x| x == 2 || x == 5).count();
        let reroll_count = 2.min(bad_count);
        let kept_count = bad_count - reroll_count;
        let good_vals: Vec<u32> = sorted.iter().filter(|&&x| x != 2 && x != 5).copied().collect();
        let bad_vals: Vec<u32> = sorted.iter().filter(|&&x| x == 2 || x == 5).copied().collect();
        if bad_count == 0 {
            let sum: u32 = good_vals.iter().sum();
            bf.probabilities[(sum - bf.min_value) as usize] += 1.0;
        } else if bad_count == 1 {
            for r1 in 1u32..=6 {
                let kept_bad: Vec<u32> = bad_vals.iter().rev().take(kept_count).copied().collect();
                let sum: u32 = kept_bad.iter().sum::<u32>() + good_vals.iter().sum::<u32>() + r1;
                bf.probabilities[(sum - bf.min_value) as usize] += 1.0 / 6.0;
            }
        } else {
            for r1 in 1u32..=6 { for r2 in 1u32..=6 {
                let kept_bad: Vec<u32> = bad_vals.iter().rev().take(kept_count).copied().collect();
                let sum: u32 = kept_bad.iter().sum::<u32>() + good_vals.iter().sum::<u32>() + r1 + r2;
                bf.probabilities[(sum - bf.min_value) as usize] += 1.0 / 36.0;
            }}
        }
    }}}}}
    let total: f64 = bf.probabilities.iter().sum();
    for p in &mut bf.probabilities { *p /= total; }
    let result = get_dice_roll_distribution(&DiceRoll {
        number_of_dice: 5, dice_size: 6,
        reroll_modifier: Some(RerollIfEqual {dice_to_reroll: 2, numbers: vec![2, 5]}),
        clamping_modifier: None, keeping_result_modifier: None,
    });
    assert_eq!(result.min_value, bf.min_value);
    assert_eq!(result.probabilities.len(), bf.probabilities.len());
    for i in 0..result.probabilities.len() {
        assert!((result.probabilities[i] - bf.probabilities[i]).abs() < 1e-6,
                "value {}: bf {}, result {}", result.min_value + i as u32, bf.probabilities[i], result.probabilities[i]);
    }
}

#[test]
fn it_correctly_computes_reroll_equal_keep_highest_3() {
    // 5d6r2={2,5}kh3
    let mut bf = Distribution { probabilities: vec![0.0; 16], min_value: 3u32 };
    for a in 1u32..=6 { for b in 1u32..=6 { for c in 1u32..=6 { for d in 1u32..=6 { for e in 1u32..=6 {
        let mut sorted = vec![a, b, c, d, e]; sorted.sort();
        let bad_count = sorted.iter().filter(|&&x| x == 2 || x == 5).count();
        let reroll_count = 2.min(bad_count);
        let kept_count = bad_count - reroll_count;
        let good_vals: Vec<u32> = sorted.iter().filter(|&&x| x != 2 && x != 5).copied().collect();
        let bad_vals: Vec<u32> = sorted.iter().filter(|&&x| x == 2 || x == 5).copied().collect();
        if bad_count == 0 {
            let mut all = good_vals; all.sort_unstable_by(|x, y| y.cmp(x));
            bf.probabilities[(all[0] + all[1] + all[2] - bf.min_value) as usize] += 1.0;
        } else if bad_count == 1 {
            for r1 in 1u32..=6 {
                let kept_bad: Vec<u32> = bad_vals.iter().rev().take(kept_count).copied().collect();
                let mut final_vals: Vec<u32> = vec![r1]; final_vals.extend(&kept_bad); final_vals.extend(&good_vals);
                final_vals.sort_unstable_by(|x, y| y.cmp(x));
                bf.probabilities[(final_vals[0] + final_vals[1] + final_vals[2] - bf.min_value) as usize] += 1.0 / 6.0;
            }
        } else {
            for r1 in 1u32..=6 { for r2 in 1u32..=6 {
                let kept_bad: Vec<u32> = bad_vals.iter().rev().take(kept_count).copied().collect();
                let mut final_vals: Vec<u32> = vec![r1, r2]; final_vals.extend(&kept_bad); final_vals.extend(&good_vals);
                final_vals.sort_unstable_by(|x, y| y.cmp(x));
                bf.probabilities[(final_vals[0] + final_vals[1] + final_vals[2] - bf.min_value) as usize] += 1.0 / 36.0;
            }}
        }
    }}}}}
    let total: f64 = bf.probabilities.iter().sum();
    for p in &mut bf.probabilities { *p /= total; }
    let result = get_dice_roll_distribution(&DiceRoll {
        number_of_dice: 5, dice_size: 6,
        reroll_modifier: Some(RerollIfEqual {dice_to_reroll: 2, numbers: vec![2, 5]}),
        clamping_modifier: None, keeping_result_modifier: Some(KeepHighest {number_of_dice: 3}),
    });
    assert_eq!(result.min_value, bf.min_value);
    assert_eq!(result.probabilities.len(), bf.probabilities.len());
    for i in 0..result.probabilities.len() {
        assert!((result.probabilities[i] - bf.probabilities[i]).abs() < 1e-6,
                "value {}: bf {}, result {}", result.min_value + i as u32, bf.probabilities[i], result.probabilities[i]);
    }
}

#[test]
fn it_correctly_computes_reroll_equal_keep_lowest_3() {
    // 5d6r2={2,5}kl3
    let mut bf = Distribution { probabilities: vec![0.0; 16], min_value: 3u32 };
    for a in 1u32..=6 { for b in 1u32..=6 { for c in 1u32..=6 { for d in 1u32..=6 { for e in 1u32..=6 {
        let mut sorted = vec![a, b, c, d, e]; sorted.sort();
        let bad_count = sorted.iter().filter(|&&x| x == 2 || x == 5).count();
        let reroll_count = 2.min(bad_count);
        let kept_count = bad_count - reroll_count;
        let good_vals: Vec<u32> = sorted.iter().filter(|&&x| x != 2 && x != 5).copied().collect();
        let bad_vals: Vec<u32> = sorted.iter().filter(|&&x| x == 2 || x == 5).copied().collect();
        if bad_count == 0 {
            let mut all = good_vals; all.sort();
            bf.probabilities[(all[0] + all[1] + all[2] - bf.min_value) as usize] += 1.0;
        } else if bad_count == 1 {
            for r1 in 1u32..=6 {
                let kept_bad: Vec<u32> = bad_vals.iter().rev().take(kept_count).copied().collect();
                let mut final_vals: Vec<u32> = vec![r1]; final_vals.extend(&kept_bad); final_vals.extend(&good_vals);
                final_vals.sort();
                bf.probabilities[(final_vals[0] + final_vals[1] + final_vals[2] - bf.min_value) as usize] += 1.0 / 6.0;
            }
        } else {
            for r1 in 1u32..=6 { for r2 in 1u32..=6 {
                let kept_bad: Vec<u32> = bad_vals.iter().rev().take(kept_count).copied().collect();
                let mut final_vals: Vec<u32> = vec![r1, r2]; final_vals.extend(&kept_bad); final_vals.extend(&good_vals);
                final_vals.sort();
                bf.probabilities[(final_vals[0] + final_vals[1] + final_vals[2] - bf.min_value) as usize] += 1.0 / 36.0;
            }}
        }
    }}}}}
    let total: f64 = bf.probabilities.iter().sum();
    for p in &mut bf.probabilities { *p /= total; }
    let result = get_dice_roll_distribution(&DiceRoll {
        number_of_dice: 5, dice_size: 6,
        reroll_modifier: Some(RerollIfEqual {dice_to_reroll: 2, numbers: vec![2, 5]}),
        clamping_modifier: None, keeping_result_modifier: Some(KeepLowest {number_of_dice: 3}),
    });
    assert_eq!(result.min_value, bf.min_value);
    assert_eq!(result.probabilities.len(), bf.probabilities.len());
    for i in 0..result.probabilities.len() {
        assert!((result.probabilities[i] - bf.probabilities[i]).abs() < 1e-6,
                "value {}: bf {}, result {}", result.min_value + i as u32, bf.probabilities[i], result.probabilities[i]);
    }
}

#[test]
fn it_correctly_computes_sum_of_highest_2_with_reroll_distributions_and_min_10() {
    let d = Distribution {
        probabilities: vec![1.0 / 20.0; 20],
        min_value: 1
    };

    // brute force of keep highest 2 out of three from distribution d with 2 dice to reroll if < 6
    let mut brute_forced_probabilities = Distribution {
        probabilities: vec![0.0; d.probabilities.len() * 2 - 20 + 1],
        min_value: 20
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

                                brute_forced_probabilities.probabilities[(sorted_values[1].max(10) + sorted_values[2].max(10) - brute_forced_probabilities.min_value) as usize] += 1.0 / 400.0;
                            }
                        } else {
                            let mut sorted_values = vec![i, sorted_values[1], sorted_values[2]];
                            sorted_values.sort();

                            brute_forced_probabilities.probabilities[(sorted_values[1].max(10) + sorted_values[2].max(10) - brute_forced_probabilities.min_value) as usize] += 1.0 / 20.0;
                        }
                    }
                } else {
                    brute_forced_probabilities.probabilities[(sorted_values[1].max(10) + sorted_values[2].max(10) - brute_forced_probabilities.min_value) as usize] += 1.0;
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
        clamping_modifier: Some(Minimum {number: 10}),
        keeping_result_modifier: Some(KeepHighest {number_of_dice: 2})
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
fn it_correctly_computes_sum_of_highest_2_with_reroll_distributions_and_max_15() {
    let d = Distribution {
        probabilities: vec![1.0 / 20.0; 20],
        min_value: 1
    };

    // brute force of keep highest 2 out of three from distribution d with 2 dice to reroll if < 6
    let mut brute_forced_probabilities = Distribution {
        probabilities: vec![0.0; d.probabilities.len() * 2 - 2 + 1 - 5 * 2],
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

                                brute_forced_probabilities.probabilities[(sorted_values[1].min(15) + sorted_values[2].min(15) - brute_forced_probabilities.min_value) as usize] += 1.0 / 400.0;
                            }
                        } else {
                            let mut sorted_values = vec![i, sorted_values[1], sorted_values[2]];
                            sorted_values.sort();

                            brute_forced_probabilities.probabilities[(sorted_values[1].min(15) + sorted_values[2].min(15) - brute_forced_probabilities.min_value) as usize] += 1.0 / 20.0;
                        }
                    }
                } else {
                    brute_forced_probabilities.probabilities[(sorted_values[1].min(15) + sorted_values[2].min(15) - brute_forced_probabilities.min_value) as usize] += 1.0;
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
        clamping_modifier: Some(Maximum {number: 15}),
        keeping_result_modifier: Some(KeepHighest {number_of_dice: 2})
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
fn it_correctly_computes_sum_of_highest_2_and_min_10() {
    let d = Distribution {
        probabilities: vec![1.0 / 20.0; 20],
        min_value: 1
    };

    // brute force of keep highest 2 out of three from distribution d with 2 dice to reroll if < 6
    let mut brute_forced_probabilities = Distribution {
        probabilities: vec![0.0; d.probabilities.len() * 2 - 20 + 1],
        min_value: 20
    };

    for i in 1..=20 {
        for j in 1..=20 {
            for k in 1..=20 {
                let mut sorted_values = vec![i, j, k];
                sorted_values.sort_unstable();

                brute_forced_probabilities.probabilities[(sorted_values[1].max(10) + sorted_values[2].max(10) - brute_forced_probabilities.min_value) as usize] += 1.0;
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
        reroll_modifier: None,
        clamping_modifier: Some(Minimum {number: 10}),
        keeping_result_modifier: Some(KeepHighest {number_of_dice: 2})
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
fn it_correctly_computes_sum_of_highest_2_and_max_15() {
    let d = Distribution {
        probabilities: vec![1.0 / 20.0; 20],
        min_value: 1
    };

    // brute force of keep highest 2 out of three from distribution d with 2 dice to reroll if < 6
    let mut brute_forced_probabilities = Distribution {
        probabilities: vec![0.0; d.probabilities.len() * 2 - 2 + 1 - 5 * 2],
        min_value: 2
    };

    for i in 1..=20 {
        for j in 1..=20 {
            for k in 1..=20 {
                let mut sorted_values = vec![i, j, k];
                sorted_values.sort_unstable();

                brute_forced_probabilities.probabilities[(sorted_values[1].min(15) + sorted_values[2].min(15) - brute_forced_probabilities.min_value) as usize] += 1.0;
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
        reroll_modifier: None,
        clamping_modifier: Some(Maximum {number: 15}),
        keeping_result_modifier: Some(KeepHighest {number_of_dice: 2})
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
fn it_correctly_computes_sum_of_lowest_2_and_min_10() {
    let d = Distribution {
        probabilities: vec![1.0 / 20.0; 20],
        min_value: 1
    };

    // brute force of keep highest 2 out of three from distribution d with 2 dice to reroll if < 6
    let mut brute_forced_probabilities = Distribution {
        probabilities: vec![0.0; d.probabilities.len() * 2 - 20 + 1],
        min_value: 20
    };

    for i in 1..=20 {
        for j in 1..=20 {
            for k in 1..=20 {
                let mut sorted_values = vec![i, j, k];
                sorted_values.sort_unstable();

                brute_forced_probabilities.probabilities[(sorted_values[0].max(10) + sorted_values[1].max(10) - brute_forced_probabilities.min_value) as usize] += 1.0;
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
        reroll_modifier: None,
        clamping_modifier: Some(Minimum {number: 10}),
        keeping_result_modifier: Some(KeepLowest {number_of_dice: 2})
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
fn it_correctly_computes_sum_of_lowest_2_and_max_15() {
    let d = Distribution {
        probabilities: vec![1.0 / 20.0; 20],
        min_value: 1
    };

    // brute force of keep highest 2 out of three from distribution d with 2 dice to reroll if < 6
    let mut brute_forced_probabilities = Distribution {
        probabilities: vec![0.0; d.probabilities.len() * 2 - 2 + 1 - 5 * 2],
        min_value: 2
    };

    for i in 1..=20 {
        for j in 1..=20 {
            for k in 1..=20 {
                let mut sorted_values = vec![i, j, k];
                sorted_values.sort_unstable();

                brute_forced_probabilities.probabilities[(sorted_values[0].min(15) + sorted_values[1].min(15) - brute_forced_probabilities.min_value) as usize] += 1.0;
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
        reroll_modifier: None,
        clamping_modifier: Some(Maximum {number: 15}),
        keeping_result_modifier: Some(KeepLowest {number_of_dice: 2})
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
fn benchmark() {
    let number_of_dice = 3;
    let dice_size = 20;
    let number_to_keep = 1;
    let dice_to_reroll = 2;
    let numbers_for_reroll = vec![4, 5, 6, 14, 15, 16];
    let minimum = 0;
    let maximum = 19;

    let mut dice_roll = DiceRoll {
        number_of_dice,
        dice_size,
        reroll_modifier: None,
        clamping_modifier: None,
        keeping_result_modifier: None
    };

    if dice_to_reroll > 0 {
        dice_roll.reroll_modifier = Some(RerollIfEqual {
            dice_to_reroll,
            numbers: numbers_for_reroll.clone()
        });
    }

    if number_to_keep > 0 {
        dice_roll.keeping_result_modifier = Some(KeepHighest {
            number_of_dice: number_to_keep
        });
    }

    if minimum > 1 {
        dice_roll.clamping_modifier = Some(Minimum {
            number: minimum
        })
    } else if maximum < dice_size {
        dice_roll.clamping_modifier = Some(Maximum {
            number: maximum
        })
    }

    let start = std::time::Instant::now();
    let d = get_dice_roll_distribution(& dice_roll);
    let elapsed = start.elapsed();

    dbg!(& d);

    println!(
        "{}d{}kh{}r{}={{{}}}min{} | eval: {:>10.1?}",
        number_of_dice,
        dice_size,
        number_to_keep,
        dice_to_reroll,
        numbers_for_reroll.iter().map(|x| x.to_string() + ",").collect::<String>(),
        minimum,
        elapsed,
    );
}