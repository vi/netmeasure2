use crate::Result;

counted_array!(
pub const DELAY_DELTAS: [i16; _] = [
0, 10, -10, 20, -20, 30, -30, 40, -40, 50,
-50, 60, -60, 70, -70, 80, -80, 90, -90, 100, -100, 200, -200, 300, -300,
400, -400, 500, -500, 1000, -1000
]);

/// Hard-coded delays ranges
counted_array!(
pub const DELAY_VALUES: [u16; _] = [
    20, 50, 100, 150, 200, 250, 300, 400, 500, 600, 700, 800, 900, 1000, 1200, 1400, 1600, 1800,
    2000, 2500, 3000, 4000, 5000, 65535,
]);

#[derive(Debug,Default,Serialize,Deserialize)]
pub struct DelayModel {
    pub value_popularity: [f32; DELAY_VALUES.len()],
    pub delta_popularity: [f32; DELAY_DELTAS.len()],
}

/// Hard-coded loss (or non-loss) cluster ranges
counted_array!(
const CLUSTERS: [u16; _] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 15, 20, 25, 30, 35, 40, 45, 50, 60, 70, 80, 90, 100, 120,
    150, 200, 300, 400, 65535,
]);

#[derive(Debug,Default,Serialize,Deserialize)]
pub struct LossModel {
    pub nonloss: [f32; CLUSTERS.len()],
    pub loss: [f32; CLUSTERS.len()],
}

#[derive(Debug,Default,Serialize,Deserialize)]
pub struct ExperimentResults {
    pub delay_model: DelayModel,
    pub loss_model: LossModel,
    pub session_id: u64,
}

const ER_SIZE : usize = ::std::mem::size_of::<ExperimentResults>() * 3/2 + 64;
const_assert!(er_fits_udp_packet; ER_SIZE < 1200);

pub fn dump_some_results() -> Result<()>  {
    let mut r = ExperimentResults::default();
    let mut rnd = ::rand::thread_rng();
    use ::rand::Rng;
    for v in r.delay_model.value_popularity.iter_mut() { *v = rnd.gen(); }
    for v in r.delay_model.delta_popularity.iter_mut() { *v = rnd.gen(); }
    for v in r.loss_model.nonloss          .iter_mut() { *v = rnd.gen(); }
    for v in r.loss_model.loss             .iter_mut() { *v = rnd.gen(); }
    ::serde_cbor::ser::to_writer_sd(&mut ::std::io::stdout().lock(), &r)?;
    Ok(())
}