use crate::Result;
use ::std::rc::Rc;

/// Hard-coded values of delay difference with previous packet's delay. Must be sorted.
counted_array!(
pub const DELAY_DELTAS: [i32; _] = [
-1000, -500, -300, -200, -100, -90, -80, -70, -60, -50, -40, -30, -20, -10, -5,
0,
5, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 200, 300, 500, 1000,
]);

/// Hard-coded delays values. Must be sorted.
counted_array!(
pub const DELAY_VALUES: [i32; _] = [
    0, 10, 20, 40, 70, 100, 150, 200, 250, 300, 350, 400, 500, 600, 700, 800, 900, 1000, 1200, 1400, 1600, 1800,
    2000, 2500, 3000, 4000, 5000, 7000, 10000, 65535,
]);

#[derive(Debug,Default,Serialize,Deserialize,Clone)]
pub struct DelayModel {
    pub value_popularity: [f32; DELAY_VALUES.len()],
    pub delta_popularity: [f32; DELAY_DELTAS.len()],
    pub mean_delay_ms: f32,
}

/// Hard-coded loss (or non-loss) cluster ranges. Must be sorted.
counted_array!(
pub const CLUSTERS: [i32; _] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 15, 20, 25, 30, 35, 40, 45, 50, 60, 70, 80, 90, 100, 120,
    150, 200, 300, 400, 65535,
]);

#[derive(Debug,Default,Serialize,Deserialize,Clone)]
pub struct LossModel {
    pub nonloss: [f32; CLUSTERS.len()],
    pub loss: [f32; CLUSTERS.len()],
    pub loss_prob: f32,
    pub sendside_loss: f32,
}

#[derive(Debug,Default,Serialize,Deserialize,Clone)]
pub struct ExperimentResults {
    pub delay_model: DelayModel,
    pub loss_model: LossModel,
    pub session_id: u64,
    pub total_received_packets: u32,
}

#[derive(Debug,Serialize,Deserialize)]
pub struct ResultsForStoring {
    pub to_server: Option<Rc<ExperimentResults>>,
    pub from_server: Option<Rc<ExperimentResults>>,
    pub conditions: super::statement::ExperimentInfo,
    pub rtt_us: u32,
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
    let rpl = super::statement::ExperimentReply::HereAreResults{
        stats:Some(Rc::new(r)),
        send_lost: None,
    };
    let s2c = crate::ServerToClient::from((rpl,0));
    ::serde_cbor::ser::to_writer_sd(&mut ::std::io::stdout().lock(), &s2c)?;
    Ok(())
}