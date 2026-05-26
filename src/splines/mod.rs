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

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
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

fn get_equation_factors(equation: &mut Vec<f64>, coefficient_idx: &usize, nb_coefficients: &usize, x_value: &f64, derivative: &usize, sign: &f64) {
    let mut cur_x_value = 1.0;
    for i in *derivative..*nb_coefficients {
        let mut derivative_coeff = 1.0;
        for j in 0..*derivative {
            derivative_coeff = derivative_coeff * ((i - j) as f64);
        }

        equation[coefficient_idx + i] = 1.0 * derivative_coeff * cur_x_value * sign;

        cur_x_value *= x_value;
    }
}

fn get_graph_spline_equation_matrix(intervals: &Vec<GraphSplineInterval>) -> Vec<Vec<f64>> {
    let nb_unknowns = intervals
        .iter()
        .map(|i| i.polynomial.coefficients.len())
        .sum()
    ;

    let mut equation_matrix = vec![vec![0.0; nb_unknowns + 1]; nb_unknowns];
    let mut equation_idx = 0;
    let mut coefficient_idx = 0;
    let mut has_done_initial_boundary_condtition = false;

    // Initial boundary condition
    
    // Starts on first point
    let first_interval = intervals.first().expect("We should have at least one interval if we called this function");
    equation_matrix[equation_idx][coefficient_idx] = 1.0;
    equation_matrix[equation_idx][nb_unknowns] = first_interval.start.y;
    equation_idx += 1;

    // inner knots conditions
    intervals
        .windows(2)
        .for_each(|intervals| {
            let max_x_value_left = intervals[0].end.x - intervals[0].start.x;
            let coefficient_idx_right = coefficient_idx + intervals[0].polynomial.coefficients.len();

            // C0 continuity left interval
            get_equation_factors(
                &mut equation_matrix[equation_idx],
                &coefficient_idx,
                &intervals[0].polynomial.coefficients.len(),
                &max_x_value_left,
                &0,
                &1.0
            );
            equation_matrix[equation_idx][nb_unknowns] = intervals[0].end.y;
            equation_idx += 1;

            // Additional constraint for local optimum: first derivative must be zero
            if intervals[0].polynomial.coefficients.len() == 5 {
                get_equation_factors(
                    &mut equation_matrix[equation_idx],
                    &coefficient_idx,
                    &intervals[0].polynomial.coefficients.len(),
                    &max_x_value_left,
                    &1,
                    &1.0
                );
                equation_idx += 1;
            }

            // C1 continuity
            let derivative_number = 1;
            get_equation_factors(
                &mut equation_matrix[equation_idx],
                &coefficient_idx,
                &intervals[0].polynomial.coefficients.len(),
                &max_x_value_left,
                &derivative_number,
                &1.0
            );
            get_equation_factors(
                &mut equation_matrix[equation_idx],
                &coefficient_idx_right,
                &intervals[1].polynomial.coefficients.len(),
                &0.0,
                &derivative_number,
                &-1.0
            );
            equation_idx += 1;

            if !has_done_initial_boundary_condtition {
                // Natural spline
                equation_matrix[equation_idx][coefficient_idx + 2] = 2.0;
                equation_idx += 1;

                has_done_initial_boundary_condtition = true;
            }

            // C2 continuity
            let derivative_number = 2;
            get_equation_factors(
                &mut equation_matrix[equation_idx],
                &coefficient_idx,
                &intervals[0].polynomial.coefficients.len(),
                &max_x_value_left,
                &derivative_number,
                &1.0
            );
            get_equation_factors(
                &mut equation_matrix[equation_idx],
                &coefficient_idx_right,
                &intervals[1].polynomial.coefficients.len(),
                &0.0,
                &derivative_number,
                &-1.0
            );
            equation_idx += 1;

            // C0 continuity right interval
            get_equation_factors(
                &mut equation_matrix[equation_idx],
                &coefficient_idx_right,
                &intervals[1].polynomial.coefficients.len(),
                &0.0,
                &0,
                &1.0
            );
            equation_matrix[equation_idx][nb_unknowns] = intervals[1].start.y;
            equation_idx += 1;

            coefficient_idx += intervals[0].polynomial.coefficients.len();
        })
    ;

    // Ends on last point
    let last_interval = intervals.last().expect("We should have at least one interval if we called this function");
    let max_x_value = last_interval.end.x - last_interval.start.x;
    get_equation_factors(
        &mut equation_matrix[equation_idx],
        &coefficient_idx,
        &last_interval.polynomial.coefficients.len(),
        &max_x_value,
        &0,
        &1.0
    );
    equation_matrix[equation_idx][nb_unknowns] = last_interval.end.y;
    equation_idx += 1;

    // Final boundary conditions
    // Natural spline
    get_equation_factors(
        &mut equation_matrix[equation_idx],
        &coefficient_idx,
        &last_interval.polynomial.coefficients.len(),
        &max_x_value,
        &2,
        &1.0
    );

    equation_matrix
}

fn reorder_equation_matrix(equation_matrix: &mut Vec<Vec<f64>>) {
    equation_matrix.sort_by(|a, b| {
        let a_left_index = a
            .iter()
            .position(|e| *e != 0.0)
            .unwrap_or_default()
        ;

        let b_left_index = b
            .iter()
            .position(|e| *e != 0.0)
            .unwrap_or_default()
        ;

        if a_left_index < b_left_index {
            return Ordering::Less;
        } else if a_left_index > b_left_index {
            return Ordering::Greater;
        }

        let a_right_index = a
            .iter()
            .rev()
            .skip(1)
            .position(|e| *e != 0.0)
            .unwrap_or_default()
        ;

        let b_right_index = b
            .iter()
            .rev()
            .skip(1)
            .position(|e| *e != 0.0)
            .unwrap_or_default()
        ;

        if a_right_index > b_right_index {
            return Ordering::Less;
        } else if a_right_index < b_right_index {
            return Ordering::Greater;
        }

        return Ordering::Equal;
    });
}

#[derive(Debug)]
struct SolvedindexLedger {
    solved_indices: HashSet<usize>,
    cur_valid_index: usize
}

impl SolvedindexLedger {
    fn find_next_valid_index_increasing(&mut self) {
        while self.solved_indices.contains(&self.cur_valid_index) {
            self.cur_valid_index += 1;
        }
    }

    fn find_next_valid_index_decreasing(&mut self) {
        while self.solved_indices.contains(&self.cur_valid_index) {
            self.cur_valid_index -= 1;
        }
    }
}

fn solve_equation_matrix(equation_matrix: &mut Vec<Vec<f64>>) {
    // 4 columns, 7 rows
    
    // First pass, top to bottom
    let mut ledger = SolvedindexLedger {
        solved_indices: HashSet::new(),
        cur_valid_index: 0
    };
    
    let nb_equations = equation_matrix.len();

    for cur_equation_idx in 0..nb_equations - 1 {
        // Using the ledger to avoid having to scan through all the indices that we know are zeros
        // by construction. This should reduce time complexity by one degree
        ledger.find_next_valid_index_increasing();
        let equation_first_non_zero_idx = ledger.cur_valid_index + equation_matrix[cur_equation_idx][ledger.cur_valid_index..]
            .iter()
            .position(|e| *e != 0.0)
            .expect("We should always have at least one non-zero value by equation")
        ;
        
        // By construction, we only have to check 6 rows below cur_equation_idx 
        // and 4 columns to the right of equation_first_non_zero_idx
        // Being able to use a fixed size matrix shaves off 2 degrees from the time complexity
        let mut has_found_non_zero_value = false;
        for equation_to_eliminate_idx in (cur_equation_idx + 1)..min(cur_equation_idx + 7, nb_equations) {
            if equation_matrix[equation_to_eliminate_idx][equation_first_non_zero_idx] != 0.0 {
                has_found_non_zero_value = true;
                
                let mult_coeff = equation_matrix[equation_to_eliminate_idx][equation_first_non_zero_idx] / equation_matrix[cur_equation_idx][equation_first_non_zero_idx];
                
                for coeff_idx in equation_first_non_zero_idx..min(equation_first_non_zero_idx + 4, nb_equations) {
                    if equation_matrix[cur_equation_idx][coeff_idx] != 0.0 {
                        equation_matrix[equation_to_eliminate_idx][coeff_idx] -= equation_matrix[cur_equation_idx][coeff_idx] * mult_coeff;
                    }
                }
                
                // We also take into account the last column, which is the other side of the equation
                if equation_matrix[cur_equation_idx][nb_equations] != 0.0 {
                    equation_matrix[equation_to_eliminate_idx][nb_equations] -= 
                        equation_matrix[cur_equation_idx][nb_equations] * mult_coeff;
                }
            } else if has_found_non_zero_value {
                // By construction, we know that all equations with a non-zero equation_first_non_zero_idx
                // are next to each other, so we stop scanning rows at the first zero value
                // encountered after the first non-zero value
                break;
            }
        }
        
        // Mark the index in the ledger
        ledger.solved_indices.insert(equation_first_non_zero_idx);
    }
    
    // Second pass: bottom to top
    let mut ledger = SolvedindexLedger {
        solved_indices: HashSet::new(),
        cur_valid_index: nb_equations - 1
    };

    for cur_equation_idx in (1..nb_equations).rev() {
        ledger.find_next_valid_index_decreasing();
        let equation_last_non_zero_idx = ledger.cur_valid_index - equation_matrix[cur_equation_idx][..=ledger.cur_valid_index]
            .iter().rev()
            .position(|e| *e != 0.0)
            .expect("We should always have at least one non-zero value by equation")
        ;
        
        // We normalize the current line if needed
        if equation_matrix[cur_equation_idx][equation_last_non_zero_idx] != 1.0 {
            equation_matrix[cur_equation_idx][nb_equations] /= equation_matrix[cur_equation_idx][equation_last_non_zero_idx];
            equation_matrix[cur_equation_idx][equation_last_non_zero_idx] = 1.0;
        }

        // By construction, we now only have to check 4 rows above cur_equation_idx 
        // and only the same column as equation_first_non_zero_idx
        for equation_to_eliminate_idx in cur_equation_idx.saturating_sub(4)..cur_equation_idx {
            if equation_matrix[equation_to_eliminate_idx][equation_last_non_zero_idx] != 0.0 {
                let mult_coeff = equation_matrix[equation_to_eliminate_idx][equation_last_non_zero_idx] / equation_matrix[cur_equation_idx][equation_last_non_zero_idx];
                
                equation_matrix[equation_to_eliminate_idx][equation_last_non_zero_idx] -= 
                    equation_matrix[cur_equation_idx][equation_last_non_zero_idx] * mult_coeff;

                // We also take into account the last column, which is the other side of the equation
                if equation_matrix[cur_equation_idx][nb_equations] != 0.0 {
                    equation_matrix[equation_to_eliminate_idx][nb_equations] -= equation_matrix[cur_equation_idx][nb_equations] * mult_coeff;
                }
            }
        }

        // Mark the index in the ledger
        ledger.solved_indices.insert(equation_last_non_zero_idx);
    }
    
    reorder_equation_matrix(equation_matrix);
}

fn apply_matrix_solution_to_intervals(equation_matrix: & Vec<Vec<f64>>, intervals: &mut Vec<GraphSplineInterval>) {
    let mut cur_offset = 0;
    let solution_idx = equation_matrix[0].len() - 1;

    for cur_interval in intervals {
        let nb_coeffs = cur_interval.polynomial.coefficients.len();

        for i in 0..nb_coeffs {
            cur_interval.polynomial.coefficients[i] = equation_matrix[cur_offset + i][solution_idx]
        }

        cur_offset += nb_coeffs
    }
}

pub fn get_graph_spline_interpolation_function(points: impl Into<Vec<Point>>) -> Option<GraphSpline> {
    let points: Vec<Point> = points.into();

    if points.len() < 2 {
        return None;
    }

    // We're going to construct the equation system from the points
    let mut intervals = get_graph_spline_intervals(&points);
    let mut equation_matrix = get_graph_spline_equation_matrix(&intervals);

    // We solve the equation
    solve_equation_matrix(&mut equation_matrix);

    // Finally, we report the solution into the final object
    apply_matrix_solution_to_intervals(&equation_matrix, &mut intervals);

    Some(GraphSpline {
        intervals
    })
}

pub fn get_graph_spline_interpolation_function_v2(points: impl Into<Vec<Point>>) -> Option<GraphSpline> {
    let points: Vec<Point> = points.into();

    if points.len() < 2 {
        return None;
    }

    None
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
    fn it_correctly_generates_equation_matrix_for_simple_case() {
        let points = vec![
            Point {x: 0.0, y: 1.0},
            Point {x: 1.0, y: 3.0},
            Point {x: 2.0, y: 2.0}
        ];

        let intervals = get_graph_spline_intervals(&points);

        let equation_matrix = get_graph_spline_equation_matrix(&intervals);
        assert_eq!(equation_matrix, vec![
            vec![1.0,   0.0,   0.0,   0.0,   0.0,   0.0,   0.0,   0.0,   0.0,   1.0],
            vec![1.0,   1.0,   1.0,   1.0,   1.0,   0.0,   0.0,   0.0,   0.0,   3.0],
            vec![0.0,   1.0,   2.0,   3.0,   4.0,   0.0,   0.0,   0.0,   0.0,   0.0],
            vec![0.0,   1.0,   2.0,   3.0,   4.0,   0.0,  -1.0,   0.0,   0.0,   0.0],
            vec![0.0,   0.0,   2.0,   0.0,   0.0,   0.0,   0.0,   0.0,   0.0,   0.0],
            vec![0.0,   0.0,   2.0,   6.0,  12.0,   0.0,   0.0,  -2.0,   0.0,   0.0],
            vec![0.0,   0.0,   0.0,   0.0,   0.0,   1.0,   0.0,   0.0,   0.0,   3.0],
            vec![0.0,   0.0,   0.0,   0.0,   0.0,   1.0,   1.0,   1.0,   1.0,   2.0],
            vec![0.0,   0.0,   0.0,   0.0,   0.0,   0.0,   0.0,   2.0,   6.0,   0.0],
        ]);
    }

    #[test]
    fn it_correctly_generates_equation_matrix_for_bigger_case() {
        let points = vec![
            Point {x: 0.0, y: 2.0},
            Point {x: 2.0, y: 4.0},//
            Point {x: 3.0, y: 1.0},//
            Point {x: 4.0, y: 3.0},
            Point {x: 5.0, y: 5.0},//
            Point {x: 6.0, y: 2.0}
        ];

        let intervals = get_graph_spline_intervals(&points);
        let equation_matrix = get_graph_spline_equation_matrix(&intervals);
        assert_eq!(equation_matrix, vec![
            vec![1.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      2.0],

            vec![1.0,   2.0,   4.0,   8.0,  16.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      4.0],
            vec![0.0,   1.0,   4.0,  12.0,  32.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0],
            vec![0.0,   1.0,   4.0,  12.0,  32.0,      0.0,  -1.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0],
            vec![0.0,   0.0,   2.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0],
            vec![0.0,   0.0,   2.0,  12.0,  48.0,      0.0,   0.0,  -2.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0],
            vec![0.0,   0.0,   0.0,   0.0,   0.0,      1.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      4.0],

            vec![0.0,   0.0,   0.0,   0.0,   0.0,      1.0,   1.0,   1.0,   1.0,   1.0,      0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      1.0],
            vec![0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   1.0,   2.0,   3.0,   4.0,      0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0],
            vec![0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   1.0,   2.0,   3.0,   4.0,      0.0,  -1.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0],
            vec![0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   2.0,   6.0,  12.0,      0.0,   0.0,  -2.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0],
            vec![0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      1.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      1.0],

            vec![0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      1.0,   1.0,   1.0,   1.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      3.0],
            vec![0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   1.0,   2.0,   3.0,      0.0,  -1.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0],
            vec![0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   2.0,   6.0,      0.0,   0.0,  -2.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0],
            vec![0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      1.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      3.0],

            vec![0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      1.0,   1.0,   1.0,   1.0,   1.0,      0.0,   0.0,   0.0,   0.0,      5.0],
            vec![0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0,   1.0,   2.0,   3.0,   4.0,      0.0,   0.0,   0.0,   0.0,      0.0],
            vec![0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0,   1.0,   2.0,   3.0,   4.0,      0.0,  -1.0,   0.0,   0.0,      0.0],
            vec![0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   2.0,   6.0,  12.0,      0.0,   0.0,  -2.0,   0.0,      0.0],
            vec![0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      1.0,   0.0,   0.0,   0.0,      5.0],

            vec![0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      1.0,   1.0,   1.0,   1.0,      2.0],
            vec![0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   0.0,   0.0,   0.0,      0.0,   0.0,   2.0,   6.0,      0.0],
        ]);
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

        let solution = get_graph_spline_interpolation_function(points);

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
    fn it_correctly_solves_the_equation_matrix_with_version_2() {
        let points = vec![
            Point {x: 0.0, y: 2.0},
            Point {x: 1.0, y: 3.0},
            Point {x: 2.0, y: 4.0},
            Point {x: 3.0, y: 1.0},
            Point {x: 4.0, y: 3.0},
            Point {x: 5.0, y: 5.0},
            Point {x: 6.0, y: 2.0}
        ];

        let solution = get_graph_spline_interpolation_function(& points);
        let solution_v2 = get_graph_spline_interpolation_function_v2(& points);

        assert_eq!(solution, solution_v2);
    }
}
