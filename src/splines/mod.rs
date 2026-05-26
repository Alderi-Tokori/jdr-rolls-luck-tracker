use core::cmp::Ordering;
use std::collections::HashSet;
use std::cmp::min;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl From<&iced::Point> for Point {
    fn from(item: &iced::Point) -> Self {
        Point {
            x: item.x as f64,
            y: item.y as f64
        }
    }
}

pub trait PolynomialFunction {
    fn eval(&self, x: f64) -> Option<f64>;
}

#[derive(Debug, PartialEq, Clone)]
struct Polynomial {
    coefficients: Vec<f64>,
}

impl PolynomialFunction for Polynomial {
    fn eval(&self, x: f64) -> Option<f64> {
        if self.coefficients.len() == 0 {
            return None;
        }

        let mut sum = 0.0;
        let mut x_degree = 1.0;

        self.coefficients.iter().for_each(|c| {
            sum += c * x_degree;
            x_degree *= x;
        });

        Some(sum)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct GraphSplineInterval {
    pub start: Point,
    pub end: Point,
    polynomial: Polynomial,
}

impl PolynomialFunction for GraphSplineInterval {
    fn eval(&self, x: f64) -> Option<f64> {
        self.polynomial.eval(x - self.start.x)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct GraphSpline {
    pub intervals: Vec<GraphSplineInterval>
}

impl PolynomialFunction for GraphSpline {
    fn eval(&self, x: f64) -> Option<f64> {
        if self.intervals.len() == 0 {
            return None
        }

        let first_interval = self.intervals.first().unwrap();
        let last_interval = self.intervals.last().unwrap();

        if x < first_interval.end.x {
            return first_interval.eval(x);
        } else if x >= last_interval.start.x {
            return last_interval.eval(x);
        }

        self.intervals[1..self.intervals.len() - 1]
            .into_iter()
            .find(|i| {
                i.start.x <= x && i.end.x > x
            })
            .expect("We've tested the two edge cases already so we should always have an interval here, as all intervals should be next to each other")
            .eval(x)
    }
}

fn get_graph_spline_intervals(points: &Vec<Point>) -> Vec<GraphSplineInterval> {
    let mut interval_degrees_list= Vec::new();

    points
        .windows(3)
        .for_each(|points| {
            let mut degree = 3;

            if (points[0].y < points[1].y && points[2].y <= points[1].y)
                || (points[0].y > points[1].y && points[2].y >= points[1].y) {
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

#[derive(Debug, Clone)]
struct CompactRow {
    entries: Vec<(usize, f64)>,
    rhs: f64,
}

impl CompactRow {
    fn new() -> Self {
        CompactRow {
            entries: Vec::new(),
            rhs: 0.0,
        }
    }

    fn add_entry(&mut self, col: usize, value: f64) {
        match self.entries.binary_search_by_key(&col, |(c, _)| *c) {
            Ok(idx) => self.entries[idx].1 += value,
            Err(idx) => self.entries.insert(idx, (col, value)),
        }
    }
}

fn add_equation_factors_to_compact_row(
    row: &mut CompactRow,
    coefficient_idx: usize,
    nb_coefficients: usize,
    x_value: f64,
    derivative: usize,
    sign: f64,
) {
    let mut cur_x_value = 1.0;
    for i in derivative..nb_coefficients {
        let mut derivative_coeff = 1.0;
        for j in 0..derivative {
            derivative_coeff = derivative_coeff * ((i - j) as f64);
        }

        row.add_entry(coefficient_idx + i, derivative_coeff * cur_x_value * sign);
        cur_x_value *= x_value;
    }
}

fn build_compact_equation_system(intervals: &Vec<GraphSplineInterval>) -> Vec<CompactRow> {
    let mut rows = Vec::new();

    let first_interval = intervals.first().expect("We should have at least one interval if we called this function");
    let mut row = CompactRow::new();
    row.add_entry(0, 1.0);
    row.rhs = first_interval.start.y;
    rows.push(row);

    let mut coefficient_idx = 0;
    let mut has_done_initial_boundary_condtition = false;

    intervals.windows(2).for_each(|intervals_pair| {
        let max_x_value_left = intervals_pair[0].end.x - intervals_pair[0].start.x;
        let coefficient_idx_right = coefficient_idx + intervals_pair[0].polynomial.coefficients.len();

        let mut row = CompactRow::new();
        add_equation_factors_to_compact_row(
            &mut row,
            coefficient_idx,
            intervals_pair[0].polynomial.coefficients.len(),
            max_x_value_left,
            0,
            1.0,
        );
        row.rhs = intervals_pair[0].end.y;
        rows.push(row);

        if intervals_pair[0].polynomial.coefficients.len() == 5 {
            let mut row = CompactRow::new();
            add_equation_factors_to_compact_row(
                &mut row,
                coefficient_idx,
                intervals_pair[0].polynomial.coefficients.len(),
                max_x_value_left,
                1,
                1.0,
            );
            rows.push(row);
        }

        let derivative_number = 1;
        let mut row = CompactRow::new();
        add_equation_factors_to_compact_row(
            &mut row,
            coefficient_idx,
            intervals_pair[0].polynomial.coefficients.len(),
            max_x_value_left,
            derivative_number,
            1.0,
        );
        add_equation_factors_to_compact_row(
            &mut row,
            coefficient_idx_right,
            intervals_pair[1].polynomial.coefficients.len(),
            0.0,
            derivative_number,
            -1.0,
        );
        rows.push(row);

        if !has_done_initial_boundary_condtition {
            let mut row = CompactRow::new();
            row.add_entry(coefficient_idx + 2, 2.0);
            rows.push(row);
            has_done_initial_boundary_condtition = true;
        }

        let derivative_number = 2;
        let mut row = CompactRow::new();
        add_equation_factors_to_compact_row(
            &mut row,
            coefficient_idx,
            intervals_pair[0].polynomial.coefficients.len(),
            max_x_value_left,
            derivative_number,
            1.0,
        );
        add_equation_factors_to_compact_row(
            &mut row,
            coefficient_idx_right,
            intervals_pair[1].polynomial.coefficients.len(),
            0.0,
            derivative_number,
            -1.0,
        );
        rows.push(row);

        let mut row = CompactRow::new();
        add_equation_factors_to_compact_row(
            &mut row,
            coefficient_idx_right,
            intervals_pair[1].polynomial.coefficients.len(),
            0.0,
            0,
            1.0,
        );
        row.rhs = intervals_pair[1].start.y;
        rows.push(row);

        coefficient_idx += intervals_pair[0].polynomial.coefficients.len();
    });

    let last_interval = intervals.last().expect("We should have at least one interval if we called this function");
    let max_x_value = last_interval.end.x - last_interval.start.x;
    let mut row = CompactRow::new();
    add_equation_factors_to_compact_row(
        &mut row,
        coefficient_idx,
        last_interval.polynomial.coefficients.len(),
        max_x_value,
        0,
        1.0,
    );
    row.rhs = last_interval.end.y;
    rows.push(row);

    let mut row = CompactRow::new();
    add_equation_factors_to_compact_row(
        &mut row,
        coefficient_idx,
        last_interval.polynomial.coefficients.len(),
        max_x_value,
        2,
        1.0,
    );
    rows.push(row);

    rows
}

fn apply_compact_solution_to_intervals(solution: &[f64], intervals: &mut Vec<GraphSplineInterval>) {
    let mut cur_offset = 0;

    for cur_interval in intervals {
        let nb_coeffs = cur_interval.polynomial.coefficients.len();

        for i in 0..nb_coeffs {
            cur_interval.polynomial.coefficients[i] = solution[cur_offset + i];
        }

        cur_offset += nb_coeffs;
    }
}

fn solve_banded_equation_system(rows: &[CompactRow]) -> Vec<f64> {
    let n = rows.len();

    let mut bw: usize = 0;
    for (i, row) in rows.iter().enumerate() {
        for &(col, _) in &row.entries {
            let d = if col >= i { col - i } else { i - col };
            bw = bw.max(d);
        }
    }
    let band_w = 2 * bw + 1;

    let mut band = vec![vec![0.0; band_w]; n];
    let mut rhs = vec![0.0; n];

    for (i, row) in rows.iter().enumerate() {
        for &(col, val) in &row.entries {
            let diag = col as isize - i as isize + bw as isize;
            band[i][diag as usize] = val;
        }
        rhs[i] = row.rhs;
    }

    let mut solved = HashSet::new();
    let mut cur = 0usize;

    // First pass: top to bottom (identical logic to solve_equation_matrix)
    for i in 0..n - 1 {
        while solved.contains(&cur) {
            cur += 1;
        }

        let fnz = {
            let start_col = cur.max(i.saturating_sub(bw));
            let end_col = (i + bw).min(n - 1);
            (start_col..=end_col).find(|&c| {
                let d = c as isize - i as isize + bw as isize;
                d >= 0 && (d as usize) < band_w && band[i][d as usize] != 0.0
            })
        };
        let fnz = match fnz { Some(c) => c, None => continue };

        let pivot = {
            let d = (fnz as isize - i as isize + bw as isize) as usize;
            band[i][d]
        };

        let mut has_found = false;
        for j in (i + 1)..min(i + 7, n) {
            let val_below = {
                let d = (fnz as isize - j as isize + bw as isize) as usize;
                if d < band_w { band[j][d] } else { 0.0 }
            };
            if val_below != 0.0 {
                has_found = true;
                let mult = val_below / pivot;

                let c_start = fnz;
                let c_end = min(fnz + 4, n).min(i + bw + 1);
                for c in c_start..c_end {
                    let di = (c as isize - i as isize + bw as isize) as usize;
                    if di < band_w && band[i][di] != 0.0 {
                        let dj = (c as isize - j as isize + bw as isize) as usize;
                        if dj < band_w {
                            band[j][dj] -= band[i][di] * mult;
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

        solved.insert(fnz);
    }

    // Second pass: bottom to top (identical logic to solve_equation_matrix)
    solved.clear();
    cur = n - 1;

    for i in (1..n).rev() {
        while cur > 0 && solved.contains(&cur) {
            cur -= 1;
        }

        let last = {
            let end_col = (i + bw).min(n - 1).min(cur);
            let start_col = i.saturating_sub(bw);
            (start_col..=end_col).rev().find(|&c| {
                let d = c as isize - i as isize + bw as isize;
                d >= 0 && (d as usize) < band_w && band[i][d as usize] != 0.0
            })
        };
        let last = match last { Some(c) => c, None => continue };

        let pivot_val = {
            let d = (last as isize - i as isize + bw as isize) as usize;
            band[i][d]
        };
        if pivot_val != 1.0 {
            rhs[i] /= pivot_val;
            let d = (last as isize - i as isize + bw as isize) as usize;
            band[i][d] = 1.0;
        }

        let cur_rhs = rhs[i];

        for j in i.saturating_sub(4)..i {
            let val_above = {
                let d = (last as isize - j as isize + bw as isize) as usize;
                if d < band_w { band[j][d] } else { 0.0 }
            };
            if val_above != 0.0 {
                let mult = val_above; // pivot is 1.0
                let d = (last as isize - j as isize + bw as isize) as usize;
                if d < band_w {
                    band[j][d] = 0.0;
                }
                if cur_rhs != 0.0 {
                    rhs[j] -= cur_rhs * mult;
                }
            }
        }

        solved.insert(last);
    }

    // Sort rows by first non-zero (like reorder_equation_matrix)
    let mut indexed: Vec<(usize, usize)> = (0..n).map(|i| {
        let first = (i.saturating_sub(bw)..=((i + bw).min(n - 1)))
            .find(|&c| {
                let d = c as isize - i as isize + bw as isize;
                d >= 0 && (d as usize) < band_w && band[i][d as usize] != 0.0
            })
            .unwrap_or(0);
        (i, first)
    }).collect();

    indexed.sort_by_key(|(_, first)| *first);

    (0..n).map(|pos| rhs[indexed[pos].0]).collect()
}

pub fn get_graph_spline_interpolation_function_v3(points: &[Point]) -> Option<GraphSpline> {
    if points.len() < 2 {
        return None;
    }

    let points = points.to_vec();

    let mut intervals = get_graph_spline_intervals(&points);
    let rows = build_compact_equation_system(&intervals);

    let solution = solve_banded_equation_system(&rows);

    apply_compact_solution_to_intervals(&solution, &mut intervals);

    Some(GraphSpline { intervals })
}

// We put everything into a one dimensional array to maximize cache hits
struct Compact1DBandMatrix {
    n: usize,
    bw: usize,
    band_width: usize,
    band: Vec<f64>,
    rhs: Vec<f64>,
}

fn build_compact_equation_system_v4(intervals: &Vec<GraphSplineInterval>) -> Compact1DBandMatrix {
    let rows = build_compact_equation_system(intervals);
    let n = rows.len();

    let mut bw: usize = 0;
    for (i, row) in rows.iter().enumerate() {
        for &(col, _) in &row.entries {
            let d = if col >= i { col - i } else { i - col };
            bw = bw.max(d);
        }
    }
    let band_width = 2 * bw + 1;

    let mut band = vec![0.0; n * band_width];
    let mut rhs = vec![0.0; n];

    for (i, row) in rows.iter().enumerate() {
        for &(col, val) in &row.entries {
            let diag = col as isize - i as isize + bw as isize;
            band[i * band_width + diag as usize] = val;
        }
        rhs[i] = row.rhs;
    }

    Compact1DBandMatrix { n, bw, band_width, band, rhs }
}

fn solve_1d_banded(mat: &Compact1DBandMatrix) -> Vec<f64> {
    let n = mat.n;
    let bw = mat.bw;
    let band_w = mat.band_width;
    let mut band = mat.band.clone();
    let mut rhs = mat.rhs.clone();

    let mut solved: HashSet<usize> = HashSet::new();
    let mut cur: usize = 0;

    // First pass: top to bottom
    for i in 0..n - 1 {
        while solved.contains(&cur) {
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
        for j in (i + 1)..min(i + 7, n) {
            let base_j = j * band_w;
            let val_below = {
                let d = (fnz as isize - j as isize + bw as isize) as usize;
                if d < band_w { band[base_j + d] } else { 0.0 }
            };
            if val_below != 0.0 {
                has_found = true;
                let mult = val_below / pivot;

                let c_start = fnz;
                let c_end = min(fnz + 4, n).min(i + bw + 1);
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

        solved.insert(fnz);
    }

    // Second pass: bottom to top
    solved.clear();
    cur = n - 1;

    for i in (1..n).rev() {
        while cur > 0 && solved.contains(&cur) {
            cur -= 1;
        }

        let base_i = i * band_w;
        let last = {
            let end_col = (i + bw).min(n - 1).min(cur);
            let start_col = i.saturating_sub(bw);
            (start_col..=end_col).rev().find(|&c| {
                let d = c as isize - i as isize + bw as isize;
                d >= 0 && (d as usize) < band_w && band[base_i + d as usize] != 0.0
            })
        };
        let last = match last { Some(c) => c, None => continue };

        let pivot_val = {
            let d = (last as isize - i as isize + bw as isize) as usize;
            band[base_i + d]
        };
        if pivot_val != 1.0 {
            rhs[i] /= pivot_val;
            let d = (last as isize - i as isize + bw as isize) as usize;
            band[base_i + d] = 1.0;
        }

        let cur_rhs = rhs[i];

        for j in i.saturating_sub(4)..i {
            let base_j = j * band_w;
            let val_above = {
                let d = (last as isize - j as isize + bw as isize) as usize;
                if d < band_w { band[base_j + d] } else { 0.0 }
            };
            if val_above != 0.0 {
                let mult = val_above;
                let d = (last as isize - j as isize + bw as isize) as usize;
                if d < band_w {
                    band[base_j + d] = 0.0;
                }
                if cur_rhs != 0.0 {
                    rhs[j] -= cur_rhs * mult;
                }
            }
        }

        solved.insert(last);
    }

    // Sort rows by first non-zero (like reorder_equation_matrix)
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

pub fn get_graph_spline_interpolation_function_v4(points: &[Point]) -> Option<GraphSpline> {
    if points.len() < 2 {
        return None;
    }

    let points = points.to_vec();

    let mut intervals = get_graph_spline_intervals(&points);
    let matrix = build_compact_equation_system_v4(&intervals);

    let solution = solve_1d_banded(&matrix);

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
        assert_eq!(intervals[4].polynomial.coefficients.len(), 5);
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

        let solution = get_graph_spline_interpolation_function_v4(&points);

        assert_eq!(solution, Some(
            GraphSpline {
                intervals: vec![
                    GraphSplineInterval {
                        start: Point {x: 0.0, y: 2.0},
                        end: Point {x: 1.0, y: 3.0},
                        polynomial: Polynomial {coefficients: vec![2.0, 2.3000000000000003, 0.0, -1.3000000000000003]}
                    },
                    GraphSplineInterval {
                        start: Point {x: 1.0, y: 3.0},
                        end: Point {x: 2.0, y: 4.0},
                        polynomial: Polynomial {coefficients: vec![3.0, -1.6000000000000008, -3.9000000000000012, 16.6, -10.1]}
                    },
                    GraphSplineInterval {
                        start: Point {x: 2.0, y: 4.0},
                        end: Point {x: 3.0, y: 1.0},
                        polynomial: Polynomial {coefficients: vec![4.0, 0.0, -14.7, 17.4, -5.7]}
                    },
                    GraphSplineInterval {
                        start: Point {x: 3.0, y: 1.0},
                        end: Point {x: 4.0, y: 3.0},
                        polynomial: Polynomial {coefficients: vec![1.0, 0.0, 3.3, -1.2999999999999998]}
                    },
                    GraphSplineInterval {
                        start: Point {x: 4.0, y: 3.0},
                        end: Point {x: 5.0, y: 5.0},
                        polynomial: Polynomial {coefficients: vec![3.0, 2.7, -0.5999999999999998, 1.0999999999999996, -1.2]}
                    },
                    GraphSplineInterval {
                        start: Point {x: 5.0, y: 5.0},
                        end: Point {x: 6.0, y: 2.0},
                        polynomial: Polynomial {coefficients: vec![5.0, 0.0, -4.5, 1.5]}
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

        let solution_v3 = get_graph_spline_interpolation_function_v3(&points);
        let solution_v4 = get_graph_spline_interpolation_function_v4(&points);

        assert_eq!(solution_v3, solution_v4);
    }

    #[test]
    fn benchmarks() {
        for size in &[10, 100, 1000, 10000, 100000] {
            let points: Vec<Point> = (0..*size)
                .map(|i| {
                    let x = i as f64 * 0.5;
                    let y = (x * 0.3).sin() * 100.0 + i as f64 % 7.0 * 3.0;
                    Point { x, y }
                })
                .collect();

            let start = std::time::Instant::now();
            let solution_v3 = get_graph_spline_interpolation_function_v3(&points);
            let elapsed_v3 = start.elapsed();

            let start = std::time::Instant::now();
            let solution_v4 = get_graph_spline_interpolation_function_v4(&points);
            let elapsed_v4 = start.elapsed();

            assert_eq!(solution_v3, solution_v4, "v1 and v4 differ for size {}", size);

            println!(
                "size={:>6} | v3: {:>10.1?} | v4: {:>10.1?}",
                size,
                elapsed_v3,
                elapsed_v4,
            );
        }
    }
}
