use std::cmp::min;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl From<&iced::Point> for Point {
    fn from(item: &iced::Point) -> Self {
        Point {
            x: item.x,
            y: item.y,
        }
    }
}

pub trait PolynomialFunction {
    fn eval(&self, x: f32) -> Option<f32>;
}

#[derive(Debug, PartialEq, Clone)]
struct Polynomial {
    coefficients: Vec<f32>,
}

impl PolynomialFunction for Polynomial {
    fn eval(&self, x: f32) -> Option<f32> {
        if self.coefficients.is_empty() {
            return None;
        }
        let mut result = 0.0;
        for &c in self.coefficients.iter().rev() {
            result = result * x + c;
        }
        Some(result)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct GraphSplineInterval {
    pub start: Point,
    pub end: Point,
    polynomial: Polynomial,
}

impl PolynomialFunction for GraphSplineInterval {
    fn eval(&self, x: f32) -> Option<f32> {
        self.polynomial.eval(x - self.start.x)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct GraphSpline {
    pub intervals: Vec<GraphSplineInterval>
}

impl PolynomialFunction for GraphSpline {
    fn eval(&self, x: f32) -> Option<f32> {
        if self.intervals.is_empty() {
            return None;
        }

        let first = &self.intervals[0];
        let last = self.intervals.last().unwrap();

        if x < first.end.x {
            return first.eval(x);
        }
        if x >= last.start.x {
            return last.eval(x);
        }

        let idx = self.intervals.partition_point(|i| i.start.x <= x);
        self.intervals[idx - 1].eval(x)
    }
}

fn get_graph_spline_intervals(points: &[Point]) -> Vec<GraphSplineInterval> {
    let mut interval_degrees_list= Vec::new();

    points
        .windows(3)
        .for_each(|points| {
            let mut degree = 3;

            if (points[0].y < points[1].y && points[2].y < points[1].y)
                || (points[0].y > points[1].y && points[2].y > points[1].y) {
                // To accomodate for the additionnal constraint at local optimum, the splines leading to a local
                // optimum will need to be quartic instead of only cubic
                degree += 1;
            }

            interval_degrees_list.push(GraphSplineInterval {
                start: points[0],
                end: points[1],
                polynomial: Polynomial {
                    coefficients: vec![0.0; degree + 1]
                }
            })
        })
    ;

    let nb_points = points.len();
    if nb_points >= 2 {
        interval_degrees_list.push(GraphSplineInterval {
            start: points[points.len() - 2],
            end: points[points.len() - 1],
            polynomial: Polynomial {
                coefficients: vec![0.0; 4]
            }
        })
    }

    interval_degrees_list
}

const MAX_ENTRIES_PER_ROW: usize = 10;

struct StackRow {
    entries: [(usize, f32); MAX_ENTRIES_PER_ROW],
    len: usize,
    rhs: f32,
}

impl StackRow {
    fn new() -> Self {
        StackRow {
            entries: [(0, 0.0); MAX_ENTRIES_PER_ROW],
            len: 0,
            rhs: 0.0,
        }
    }

    fn clear(&mut self, rhs: f32) {
        self.len = 0;
        self.rhs = rhs;
    }

    fn add_entry(&mut self, col: usize, value: f32) {
        self.entries[self.len] = (col, value);
        self.len += 1;
    }

    fn as_slice(&self) -> &[(usize, f32)] {
        &self.entries[..self.len]
    }
}

const FALLING_FACTORIAL: [[f32; 3]; 6] = [
    [1.0, 0.0, 0.0],
    [1.0, 1.0, 0.0],
    [1.0, 2.0, 2.0],
    [1.0, 3.0, 6.0],
    [1.0, 4.0, 12.0],
    [1.0, 5.0, 20.0],
];

fn add_equation_factors_to_stack_row_v2(
    row: &mut StackRow,
    coefficient_idx: usize,
    nb_coefficients: usize,
    x_value: f32,
    derivative: usize,
    sign: f32,
) {
    let ff = &FALLING_FACTORIAL;
    let mut cur_x_value = 1.0;
    for i in derivative..nb_coefficients {
        let derivative_coeff = ff[i][derivative];
        row.add_entry(coefficient_idx + i, derivative_coeff * cur_x_value * sign);
        cur_x_value *= x_value;
    }
}

fn for_each_equation_row_v2(intervals: &[GraphSplineInterval], mut on_row: impl FnMut(&StackRow)) {
    let mut row = StackRow::new();

    let first_interval = &intervals[0];
    row.clear(first_interval.start.y);
    row.add_entry(0, 1.0);
    on_row(&row);

    let mut coefficient_idx = 0;
    let mut has_done_initial_boundary_condtition = false;

    for intervals_pair in intervals.windows(2) {
        let max_x_value_left = intervals_pair[0].end.x - intervals_pair[0].start.x;
        let coefficient_idx_right = coefficient_idx + intervals_pair[0].polynomial.coefficients.len();

        row.clear(intervals_pair[0].end.y);
        add_equation_factors_to_stack_row_v2(
            &mut row, coefficient_idx,
            intervals_pair[0].polynomial.coefficients.len(),
            max_x_value_left, 0, 1.0,
        );
        on_row(&row);

        if intervals_pair[0].polynomial.coefficients.len() == 5 {
            row.clear(0.0);
            add_equation_factors_to_stack_row_v2(
                &mut row, coefficient_idx,
                intervals_pair[0].polynomial.coefficients.len(),
                max_x_value_left, 1, 1.0,
            );
            on_row(&row);
        }

        row.clear(0.0);
        add_equation_factors_to_stack_row_v2(
            &mut row, coefficient_idx,
            intervals_pair[0].polynomial.coefficients.len(),
            max_x_value_left, 1, 1.0,
        );
        add_equation_factors_to_stack_row_v2(
            &mut row, coefficient_idx_right,
            intervals_pair[1].polynomial.coefficients.len(),
            0.0, 1, -1.0,
        );
        on_row(&row);

        if !has_done_initial_boundary_condtition {
            row.clear(0.0);
            row.add_entry(coefficient_idx + 2, 2.0);
            on_row(&row);
            has_done_initial_boundary_condtition = true;
        }

        row.clear(0.0);
        add_equation_factors_to_stack_row_v2(
            &mut row, coefficient_idx,
            intervals_pair[0].polynomial.coefficients.len(),
            max_x_value_left, 2, 1.0,
        );
        add_equation_factors_to_stack_row_v2(
            &mut row, coefficient_idx_right,
            intervals_pair[1].polynomial.coefficients.len(),
            0.0, 2, -1.0,
        );
        on_row(&row);

        row.clear(intervals_pair[1].start.y);
        add_equation_factors_to_stack_row_v2(
            &mut row, coefficient_idx_right,
            intervals_pair[1].polynomial.coefficients.len(),
            0.0, 0, 1.0,
        );
        on_row(&row);

        coefficient_idx += intervals_pair[0].polynomial.coefficients.len();
    }

    let last_interval = intervals.last().unwrap();
    let max_x_value = last_interval.end.x - last_interval.start.x;

    row.clear(last_interval.end.y);
    add_equation_factors_to_stack_row_v2(
        &mut row, coefficient_idx,
        last_interval.polynomial.coefficients.len(),
        max_x_value, 0, 1.0,
    );
    on_row(&row);

    row.clear(0.0);
    add_equation_factors_to_stack_row_v2(
        &mut row, coefficient_idx,
        last_interval.polynomial.coefficients.len(),
        max_x_value, 2, 1.0,
    );
    on_row(&row);
}

fn apply_compact_solution_to_intervals(solution: &[f32], intervals: &mut Vec<GraphSplineInterval>) {
    let mut cur_offset = 0;

    for cur_interval in intervals {
        let nb_coeffs = cur_interval.polynomial.coefficients.len();

        for i in 0..nb_coeffs {
            cur_interval.polynomial.coefficients[i] = solution[cur_offset + i];
        }

        cur_offset += nb_coeffs;
    }
}

// We put everything into a one dimensional array to maximize cache hits
struct Compact1DBandMatrix {
    n: usize,
    bw: usize,
    band_width: usize,
    band: Vec<f32>,
    rhs: Vec<f32>,
}

fn compute_equation_metadata(intervals: &[GraphSplineInterval]) -> (usize, usize) {
    let mut n: usize = 1;
    let mut bw: usize = 0;
    let mut coeff_idx: usize = 0;
    let mut row_idx: usize = 1;

    for pair in intervals.windows(2) {
        let len_left = pair[0].polynomial.coefficients.len();
        let len_right = pair[1].polynomial.coefficients.len();
        let c1 = coeff_idx + len_left;

        let min_c = coeff_idx;
        let max_c = coeff_idx + len_left - 1;
        bw = bw.max(row_idx.abs_diff(min_c)).max(row_idx.abs_diff(max_c));
        row_idx += 1;
        n += 1;

        if len_left == 5 {
            let min_c = coeff_idx + 1;
            let max_c = coeff_idx + len_left - 1;
            bw = bw.max(row_idx.abs_diff(min_c)).max(row_idx.abs_diff(max_c));
            row_idx += 1;
            n += 1;
        }

        let min_c = coeff_idx + 1;
        let max_c = c1 + len_right - 1;
        bw = bw.max(row_idx.abs_diff(min_c)).max(row_idx.abs_diff(max_c));
        row_idx += 1;
        n += 1;

        if coeff_idx == 0 {
            let col = coeff_idx + 2;
            bw = bw.max(row_idx.abs_diff(col));
            row_idx += 1;
            n += 1;
        }

        let min_c = coeff_idx + 2;
        let max_c = c1 + len_right - 1;
        bw = bw.max(row_idx.abs_diff(min_c)).max(row_idx.abs_diff(max_c));
        row_idx += 1;
        n += 1;

        let min_c = c1;
        let max_c = c1 + len_right - 1;
        bw = bw.max(row_idx.abs_diff(min_c)).max(row_idx.abs_diff(max_c));
        row_idx += 1;
        n += 1;

        coeff_idx += len_left;
    }

    let last = intervals.last().unwrap();
    let len_last = last.polynomial.coefficients.len();

    let min_c = coeff_idx;
    let max_c = coeff_idx + len_last - 1;
    bw = bw.max(row_idx.abs_diff(min_c)).max(row_idx.abs_diff(max_c));
    row_idx += 1;
    n += 1;

    let min_c = coeff_idx + 2;
    let max_c = coeff_idx + len_last - 1;
    bw = bw.max(row_idx.abs_diff(min_c)).max(row_idx.abs_diff(max_c));
    row_idx += 1;
    n += 1;

    (n, bw)
}

fn build_compact_equation_system_v9(intervals: &Vec<GraphSplineInterval>) -> Compact1DBandMatrix {
    let (n, bw) = compute_equation_metadata(intervals);

    let band_width = 2 * bw + 1;
    let mut band = vec![0.0; n * band_width];
    let mut rhs = vec![0.0; n];

    let mut i: usize = 0;
    for_each_equation_row_v2(intervals, |row| {
        for &(col, val) in row.as_slice() {
            let diag = col as isize - i as isize + bw as isize;
            band[i * band_width + diag as usize] = val;
        }
        rhs[i] = row.rhs;
        i += 1;
    });

    Compact1DBandMatrix { n, bw, band_width, band, rhs }
}

fn solve_1d_banded_v11(mat: Compact1DBandMatrix) -> Vec<f32> {
    let n = mat.n;
    let bw = mat.bw;
    let band_w = mat.band_width;
    let mut band = mat.band;
    let mut rhs = mat.rhs;

    let mut solved_gen = vec![0u8; n];
    let generation: u8 = 1;
    let mut cur: usize = 0;

    for i in 0..n - 1 {
        while cur < n && solved_gen[cur] == generation {
            cur += 1;
        }

        let base_i = i * band_w;
        let fnz = {
            let start_col = cur.max(i.saturating_sub(bw));
            let end_col = (i + bw).min(n - 1);
            (start_col..=end_col).find(|&c| {
                let d = c as isize - i as isize + bw as isize;
                d >= 0 && (d as usize) < band_w && band[base_i + d as usize] != 0.0
            })
        };
        let fnz = match fnz { Some(c) => c, None => continue };

        let pivot = {
            let d = (fnz as isize - i as isize + bw as isize) as usize;
            band[base_i + d]
        };

        let mut has_found = false;
        for j in (i + 1)..min(i + 2 * bw + 1, n) {
            let base_j = j * band_w;
            let val_below = {
                let d = (fnz as isize - j as isize + bw as isize) as usize;
                if d < band_w { band[base_j + d] } else { 0.0 }
            };
            if val_below != 0.0 {
                has_found = true;
                let mult = val_below / pivot;

                let c_start = fnz;
                let c_end = (fnz + 2 * bw + 1).min(n);
                for c in c_start..c_end {
                    let di = (c as isize - i as isize + bw as isize) as usize;
                    if di < band_w && band[base_i + di] != 0.0 {
                        let dj = (c as isize - j as isize + bw as isize) as usize;
                        if dj < band_w {
                            band[base_j + dj] -= band[base_i + di] * mult;
                        }
                    }
                }

                if rhs[i] != 0.0 {
                    rhs[j] -= rhs[i] * mult;
                }
            } else if has_found {
                break;
            }
        }

        solved_gen[fnz] = generation;
    }

    // Backward elimination: from bottom, normalize pivot,
    // eliminate upward, then zero all non-pivot entries in the row.

    for i in (1..n).rev() {
        let base_i = i * band_w;

        let end_col = (i + bw).min(n - 1);
        let start_col = i.saturating_sub(bw);
        let last = (start_col..=end_col).rev().find(|&c| {
            let d = c as isize - i as isize + bw as isize;
            d >= 0 && (d as usize) < band_w && band[base_i + d as usize] != 0.0
        });

        let last = match last { Some(c) => c, None => continue };

        let pivot_val = {
            let d = (last as isize - i as isize + bw as isize) as usize;
            band[base_i + d]
        };

        let cur_rhs;
        if pivot_val.abs() < 1e-12 {
            cur_rhs = 0.0;
        } else if (pivot_val - 1.0).abs() < 1e-10 {
            cur_rhs = rhs[i];
        } else {
            rhs[i] /= pivot_val;
            let d = (last as isize - i as isize + bw as isize) as usize;
            band[base_i + d] = 1.0;
            cur_rhs = rhs[i];
        }

        // Eliminate pivot column from rows above
        for j in i.saturating_sub(2 * bw)..i {
            let base_j = j * band_w;
            let val_above = {
                let d = (last as isize - j as isize + bw as isize) as usize;
                if d < band_w { band[base_j + d] } else { 0.0 }
            };
            if val_above != 0.0 {
                let d = (last as isize - j as isize + bw as isize) as usize;
                if d < band_w {
                    band[base_j + d] = 0.0;
                }
                if cur_rhs != 0.0 {
                    rhs[j] -= cur_rhs * val_above;
                }
            }
        }

        // Zero out all non-pivot entries in this row so the
        // reordering step sees exactly one non-zero per row
        for c in start_col..=end_col {
            if c != last {
                let d = (c as isize - i as isize + bw as isize) as usize;
                if d < band_w {
                    band[base_i + d] = 0.0;
                }
            }
        }
    }

    let mut indexed: Vec<(usize, usize)> = (0..n).map(|i| {
        let base = i * band_w;
        let first = (i.saturating_sub(bw)..=((i + bw).min(n - 1)))
            .find(|&c| {
                let d = c as isize - i as isize + bw as isize;
                d >= 0 && (d as usize) < band_w && band[base + d as usize] != 0.0
            })
            .unwrap_or(0);
        (i, first)
    }).collect();

    indexed.sort_by_key(|(_, first)| *first);

    (0..n).map(|pos| rhs[indexed[pos].0]).collect()
}

pub fn get_graph_spline_interpolation_function(points: &[Point]) -> Option<GraphSpline> {
    if points.len() < 2 {
        return None;
    }

    let mut intervals = get_graph_spline_intervals(points);
    let matrix = build_compact_equation_system_v9(&intervals);

    let solution = solve_1d_banded_v11(matrix);

    apply_compact_solution_to_intervals(&solution, &mut intervals);

    Some(GraphSpline { intervals })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_correctly_evaluates_polynomial() {
        let p = Polynomial {
            coefficients: vec![5.0, 1.0, 1.0]
        };

        assert_eq!(p.eval(0.0), Some(5.0));
        assert_eq!(p.eval(10.0), Some(115.0));
    }

    #[test]
    fn it_correctly_returns_none_for_undefined_polynomial() {
        let p = Polynomial {
            coefficients: vec![]
        };

        assert_eq!(p.eval(0.0), None);
    }

    #[test]
    fn it_correctly_evaluates_piecewise_polynomial() {
        let p = GraphSpline {
            intervals: vec![
                GraphSplineInterval {
                    start: Point {x: 0.0, y: 0.0},
                    end: Point {x: 5.0, y: 0.0},
                    polynomial: Polynomial {
                        coefficients: vec![5.0, 2.0]
                    }
                },
                GraphSplineInterval {
                    start: Point {x: 5.0, y: 0.0},
                    end: Point {x: 10.0, y: 0.0},
                    polynomial: Polynomial {
                        coefficients: vec![0.0, 1.0]
                    }
                },
                GraphSplineInterval {
                    start: Point {x: 10.0, y: 0.0},
                    end: Point {x: 15.0, y: 0.0},
                    polynomial: Polynomial {
                        coefficients: vec![0.0, 1.0, 1.0]
                    }
                }
            ]
        };

        assert_eq!(p.eval(0.0), Some(5.0));
        assert_eq!(p.eval(4.99), Some(14.98));
        assert_eq!(p.eval(5.0), Some(0.0));
        assert_eq!(p.eval(9.99), Some(4.99));
        assert_eq!(p.eval(10.0), Some(0.0));
        assert_eq!(p.eval(12.0), Some(6.0));
        assert_eq!(p.eval(-1.0), Some(3.0));
        assert_eq!(p.eval(20.0), Some(110.0));
    }

    #[test]
    fn it_correctly_returns_none_if_no_interval_defined() {
        let p = GraphSpline {
            intervals: vec![]
        };

        assert_eq!(p.eval(1.0), None);
    }

    #[test]
    fn it_correctly_calculates_interval_degrees_list() {
        let points = vec![
            Point {x: 0.0, y: 10.0},
            Point {x: 1.0, y: 20.0},
            Point {x: 2.0, y: 30.0},
            Point {x: 3.0, y: 20.0},
            Point {x: 4.0, y: 10.0},
            Point {x: 5.0, y: 20.0},
            Point {x: 6.0, y: 20.0},
            Point {x: 7.0, y: 10.0},
            Point {x: 8.0, y: 0.0},
            Point {x: 9.0, y: 0.0},
            Point {x: 10.0, y: 10.0},
        ];

        let intervals = get_graph_spline_intervals(&points);

        assert_eq!(intervals[0].polynomial.coefficients.len(), 4);
        assert_eq!(intervals[1].polynomial.coefficients.len(), 5);
        assert_eq!(intervals[2].polynomial.coefficients.len(), 4);
        assert_eq!(intervals[3].polynomial.coefficients.len(), 5);
        assert_eq!(intervals[4].polynomial.coefficients.len(), 4);
        assert_eq!(intervals[5].polynomial.coefficients.len(), 4);
        assert_eq!(intervals[6].polynomial.coefficients.len(), 4);
        assert_eq!(intervals[7].polynomial.coefficients.len(), 5);
        assert_eq!(intervals[8].polynomial.coefficients.len(), 4);
        assert_eq!(intervals[9].polynomial.coefficients.len(), 4);
    }

    #[test]
    fn it_correctly_solves_the_equation_matrix() {
        let points = vec![
            Point {x: 0.0, y: 2.0},
            Point {x: 1.0, y: 3.0},
            Point {x: 2.0, y: 4.0},
            Point {x: 3.0, y: 1.0},
            Point {x: 4.0, y: 3.0},
            Point {x: 5.0, y: 5.0},
            Point {x: 6.0, y: 2.0}
        ];

        let solution = get_graph_spline_interpolation_function(&points);

        assert_eq!(solution, Some(
            GraphSpline {
                intervals: vec![
                    GraphSplineInterval {
                        start: Point {x: 0.0, y: 2.0},
                        end: Point {x: 1.0, y: 3.0},
                        polynomial: Polynomial {coefficients: vec![2.0, 2.3000007, 0.0, -1.3000007]}
                    },
                    GraphSplineInterval {
                        start: Point {x: 1.0, y: 3.0},
                        end: Point {x: 2.0, y: 4.0},
                        polynomial: Polynomial {coefficients: vec![3.0, -1.6000013, -3.900002, 16.600008, -10.100004]}
                    },
                    GraphSplineInterval {
                        start: Point {x: 2.0, y: 4.0},
                        end: Point {x: 3.0, y: 1.0},
                        polynomial: Polynomial {coefficients: vec![4.0, -0.0, -14.7, 17.4, -5.7]}
                    },
                    GraphSplineInterval {
                        start: Point {x: 3.0, y: 1.0},
                        end: Point {x: 4.0, y: 3.0},
                        polynomial: Polynomial {coefficients: vec![1.0, -0.0, 3.3, -1.3]}
                    },
                    GraphSplineInterval {
                        start: Point {x: 4.0, y: 3.0},
                        end: Point {x: 5.0, y: 5.0},
                        polynomial: Polynomial {coefficients: vec![3.0, 2.7, -0.59999996, 1.1, -1.2]}
                    },
                    GraphSplineInterval {
                        start: Point {x: 5.0, y: 5.0},
                        end: Point {x: 6.0, y: 2.0},
                        polynomial: Polynomial {coefficients: vec![5.0, -0.0, -4.5, 1.5]}
                    },
                ]
            }
        ))
    }

    #[test]
    fn it_correctly_solves_the_equation_matrix_with_all_versions() {
        let points = vec![
            Point {x: 0.0, y: 2.0},
            Point {x: 1.0, y: 3.0},
            Point {x: 2.0, y: 4.0},
            Point {x: 3.0, y: 1.0},
            Point {x: 4.0, y: 3.0},
            Point {x: 5.0, y: 5.0},
            Point {x: 6.0, y: 2.0}
        ];

        let solution = get_graph_spline_interpolation_function(&points);

        assert_eq!(solution, solution);
    }

    #[test]
    fn benchmarks_solver() {
        for size in &[10, 100, 1000, 10000, 100000] {
            let points: Vec<Point> = (0..*size)
                .map(|i| {
                    let x = i as f32 * 0.5;
                    let y = (x * 0.3).sin() * 100.0 + i as f32 % 7.0 * 3.0;
                    Point { x, y }
                })
                .collect();

            let start = std::time::Instant::now();
            let solution = get_graph_spline_interpolation_function(&points);
            let elapsed = start.elapsed();

            println!(
                "size={:>6} | current version: {:>10.1?}",
                size,
                elapsed,
            );
        }
    }

    #[test]
    fn benchmarks_eval() {
        for size in &[10, 100, 1000, 10000] {
            let points: Vec<Point> = (0..*size)
                .map(|i| {
                    let x = i as f32 * 0.5;
                    let y = (x * 0.3).sin() * 100.0 + i as f32 % 7.0 * 3.0;
                    Point { x, y }
                })
                .collect();

            let solution = get_graph_spline_interpolation_function(&points).unwrap();

            let eval_xs: Vec<f32> = solution.intervals.iter()
                .flat_map(|interval| {
                    let step = (interval.end.x - interval.start.x) / 100.0;
                    (1..=100).map(move |seg| interval.start.x + seg as f32 * step)
                })
                .collect();

            let start = std::time::Instant::now();
            for &x in &eval_xs {
                solution.eval(x).unwrap();
            }
            let elapsed = start.elapsed();

            println!(
                "size={:>6} | eval: {:>10.1?}",
                size,
                elapsed,
            );
        }
    }

    #[test]
    fn it_correctly_interpolates_15_point_set() {
        let points = vec![
            Point { x: 0.0, y: 2.0 },
            Point { x: 1.0, y: 3.0 },
            Point { x: 2.0, y: 3.0 },
            Point { x: 3.0, y: 4.0 },
            Point { x: 4.0, y: 1.0 },
            Point { x: 5.0, y: 3.0 },
            Point { x: 6.0, y: 5.0 },
            Point { x: 7.0, y: 2.0 },
            Point { x: 8.0, y: 3.2375002 },
            Point { x: 9.0, y: 3.2375002 },
            Point { x: 10.0, y: 4.3916664 },
            Point { x: 11.0, y: 2.7041667 },
            Point { x: 12.0, y: 2.7041667 },
            Point { x: 13.0, y: 2.7041667 },
            Point { x: 14.0, y: 4.258333 },
        ];

        let solution = get_graph_spline_interpolation_function(&points).unwrap();

        for pt in &points {
            let eval_y = solution.eval(pt.x).unwrap();
            assert!((eval_y - pt.y).abs() < 0.01,
                "Spline at x={} failed: expected {} got {}", pt.x, pt.y, eval_y);
        }
    }
}