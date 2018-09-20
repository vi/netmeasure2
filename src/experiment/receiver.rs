use ::std::time::Instant;

use super::results::{ExperimentResults,DelayModel,LossModel};

struct Info {
    t: Instant,
    seqn: u32,
}

pub struct PacketReceiver {
    v: Vec<Info>,
    start: Instant,
    stop: Instant,
    session_id: u64,
}

pub struct PacketReceiverParams {
    pub num_packets: u32,
    pub session_id: u64,
    pub experiment_start: Instant,
    pub experiment_stop: Instant,
}

impl PacketReceiver {
    pub fn register(&mut self, seqn: u32) {
        self.v.push(Info{t:Instant::now(), seqn});
    }

    pub fn new(prp: PacketReceiverParams) -> Self {
        PacketReceiver {
            start: prp.experiment_start,
            stop: prp.experiment_stop,
            v: Vec::with_capacity(prp.num_packets as usize),
            session_id: prp.session_id,
        }
    }

    pub fn expired(&self) -> bool {
        Instant::now() >= self.stop
    }

    pub fn analyse(&self) -> ExperimentResults {
        let delta_popularity = [0.0;31];
        let value_popularity = [0.0;24];
        let loss = [0.0;30];
        let nonloss = [0.0;30];

        let delay_model = DelayModel {
            delta_popularity,
            value_popularity,
        };
        let loss_model = LossModel {
            loss,
            nonloss,
        };
        ExperimentResults {
            session_id: self.session_id,
            loss_model,
            delay_model,
        }
    }
}