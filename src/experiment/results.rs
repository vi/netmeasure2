
counted_array!(
const DELAY_DELTAS: [i16; _] = [
0, 10, -10, 20, -20, 30, -30, 40, -40, 50,
-50, 60, -60, 70, -70, 80, -80, 90, -90, 100, -100, 200, -200, 300, -300,
400, -400, 500, -500, 1000, -1000
]);

/// Hard-coded delays ranges
counted_array!(
const DELAY_VALUES: [u16; _] = [
    20, 50, 100, 150, 200, 250, 300, 400, 500, 600, 700, 800, 900, 1000, 1200, 1400, 1600, 1800,
    2000, 2500, 3000, 4000, 5000, 65535,
]);

#[derive(Debug)]
struct DelayModel {
    value_popularity: [f32; DELAY_VALUES.len()],
    delta_popularity: [f32; DELAY_DELTAS.len()],
}

/// Hard-coded loss (or non-loss) cluster ranges
counted_array!(
const CLUSTERS: [u16; _] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 15, 20, 25, 30, 35, 40, 45, 50, 60, 70, 80, 90, 100, 120,
    150, 200, 300, 400, 65535,
]);

#[derive(Debug)]
struct LossModel {
    nonloss: [f32; CLUSTERS.len()],
    loss: [f32; CLUSTERS.len()],
}

#[derive(Debug)]
struct ExperimentResults {
    delay_model: DelayModel,
    loss_model: LossModel,
}

const ER_SIZE : usize = ::std::mem::size_of::<ExperimentResults>() * 3/2 + 64;
const_assert!(er_fits_udp_packet; ER_SIZE < 1200);

#[derive(Debug)]
pub struct ExperimentReply {
    base64_results: String,
    session_id: u64,
}