use std::process::ExitCode;

use amuxterm_web::benchmark_renderer;

const FRAME_BUDGET_MS: f64 = 16.0;
const UPDATES_PER_SECOND_TARGET: usize = 2_000;
const FRAMES: usize = 180;
const FRAME_RATE: usize = 20;

fn main() -> ExitCode {
    let updates_per_frame = UPDATES_PER_SECOND_TARGET / FRAME_RATE;
    let summary = benchmark_renderer(FRAMES, 24, 80, updates_per_frame);

    println!(
        "renderer benchmark: frames={} rows={} cols={} updates_per_frame={} p95_ms={:.3} mean_ms={:.3} max_ms={:.3}",
        summary.frames,
        summary.rows,
        summary.cols,
        summary.updates_per_frame,
        summary.p95_frame_time_ms,
        summary.mean_frame_time_ms,
        summary.max_frame_time_ms
    );
    println!(
        "target: p95 <= {:.1} ms at {} updates/sec",
        FRAME_BUDGET_MS, UPDATES_PER_SECOND_TARGET
    );

    if summary.p95_frame_time_ms <= FRAME_BUDGET_MS {
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "renderer benchmark failed: p95 {:.3} ms exceeded {:.1} ms budget",
            summary.p95_frame_time_ms, FRAME_BUDGET_MS
        );
        ExitCode::from(1)
    }
}
