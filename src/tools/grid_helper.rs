use egui_plot::{GridInput, GridMark};

// generate_marks and fill_marks_between are from the sources of
// egui_plot: https://librepvz.github.io/librePvZ/src/egui/widgets/plot/mod.rs.html#1634 ff.

/// Fill in all values between [min, max] which are a multiple of `step_size`
fn generate_marks(step_sizes: [f64; 3], bounds: (f64, f64)) -> Vec<GridMark> {
    let mut steps = vec![];
    fill_marks_between(&mut steps, step_sizes[0], bounds);
    fill_marks_between(&mut steps, step_sizes[1], bounds);
    fill_marks_between(&mut steps, step_sizes[2], bounds);
    steps
}

/// Fill in all values between [min, max] which are a multiple of `step_size`
fn fill_marks_between(out: &mut Vec<GridMark>, step_size: f64, (min, max): (f64, f64)) {
    assert!(max > min);
    let first = (min / step_size).ceil() as i64;
    let last = (max / step_size).ceil() as i64;

    let marks_iter = (first..last).map(|i| {
        let value = (i as f64) * step_size;
        GridMark { value, step_size }
    });
    out.extend(marks_iter);
}
pub fn ecg_grid_spacer(grid_input: GridInput) -> Vec<GridMark> {
    /*
    We want the classic ecg grid with
    0.04s between the smallest marks, 0.2 s between the medium marks and 1.0 s between the large marks
    but if we zoom out 10 s steps, minute, 5 minute and hour marks - so we go a little crazy
    */

    // now let's generate the grid marks based on the base_step_size
    if grid_input.base_step_size >= 60.0 {
        generate_marks([60.0, 300.0, 3600.0], grid_input.bounds)
    } else if grid_input.base_step_size >= 10.0 {
        generate_marks([10.0, 60.0, 300.0], grid_input.bounds)
    } else if grid_input.base_step_size >= 1.0 {
        generate_marks([1.0, 10.0, 60.0], grid_input.bounds)
    } else if grid_input.base_step_size >= 0.2 {
        generate_marks([0.2, 1.0, 10.0], grid_input.bounds)
    } else {
        generate_marks([0.04, 0.2, 1.0], grid_input.bounds)
    }
}
