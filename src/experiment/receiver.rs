use ::std::time::{Instant,Duration};

use super::results::{ExperimentResults,DelayModel,LossModel};

use ::byteorder::{BE,ByteOrder};

use crate::experiment::SmallishDuration;

#[derive(Copy,Clone,Default,Debug,Serialize,Deserialize)]
pub struct Info {
    seqn: u32,
    st_us: u32,
    rt_us: u32,
}

pub struct PacketReceiver {
    v: Vec<Info>,
    start: Instant,
    session_id: u64,
    ctr: usize,
}

pub struct PacketReceiverParams {
    pub num_packets: u32,
    pub session_id: u64,
    pub experiment_start: Instant,
}

impl PacketReceiver {
    pub fn recv(&mut self, pkt: &[u8]) {
        assert!(pkt.len() >= 16);
        if self.ctr >= self.v.len() { return }
        let seqn = BE::read_u32(&pkt[8..12]);
        let st_us = BE::read_u32(&pkt[12..16]);


        let recv_ts = Instant::now();
        if self.start > recv_ts { self.start = recv_ts}
        let rt_us = (recv_ts - self.start).as_us();

        self.v[self.ctr]=Info{rt_us, st_us, seqn};
        self.ctr += 1;
    }

    pub fn new(prp: PacketReceiverParams) -> Self {
        PacketReceiver {
            start: prp.experiment_start,
            // not just with_capacity to avoid page faults while filling it in
            v: vec![Default::default(); prp.num_packets as usize],
            session_id: prp.session_id,
            ctr: 0,
        }
    }

    pub fn analyse(&self) -> ExperimentResults {
        //println!("{:#?}",self.v);

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
            total_received_packets: self.ctr as u32,
        }
    }

    pub fn save_raw_data(&self, dir: &::std::path::Path) {
        if let Err(e) = (try {
            let p = dir.join(format!("{}.dat",self.session_id));
            let f = ::std::fs::File::create(p)?;
            let mut f = ::std::io::BufWriter::new(f);
            
            ::bincode::serialize_into(&mut f, &self.v.len())?;
            ::bincode::serialize_into(f, &self.v[0..self.ctr])?;
        }) {
            let e : ::failure::Error = e;
            eprintln!("Error saving raw receive data: {}", e);
        }
    }

    pub fn dump_raw_data(p: &::std::path::Path) -> crate::Result<()> {
        let mut f = ::std::io::BufReader::new(::std::fs::File::open(p)?);
        let _totpkt : usize = ::bincode::deserialize_from(&mut f)?;
        let v : Vec<Info> = ::bincode::deserialize_from(f)?;
        for inf in v {
            println!("{} {} {}", inf.seqn, inf.st_us as f64 / 1000.0, inf.rt_us as f64 / 1000.0);
        }
        Ok(())
    }
}
