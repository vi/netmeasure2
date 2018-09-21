use ::std::time::{Instant,Duration};

use super::results::{ExperimentResults,DelayModel,LossModel};

use ::byteorder::{BE,ByteOrder};

use crate::experiment::SmallishDuration;

struct Info {
    rt_us: u32,
    st_us: u32,
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
    pub fn recv(&mut self, pkt: &[u8]) {
        assert!(pkt.len() >= 16);
        let seqn = BE::read_u32(&pkt[8..12]);
        let st_us = BE::read_u32(&pkt[12..16]);


        let recv_ts = Instant::now();
        if self.start > recv_ts { self.start = recv_ts}
        let rt_us = (recv_ts - self.start).as_us();

        self.v.push(Info{rt_us, st_us, seqn});
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
            total_received_packets: self.v.len() as u32,
        }
    }
}