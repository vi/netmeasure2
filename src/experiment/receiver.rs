use ::std::time::{Duration, Instant};

use super::results::{DelayModel, ExperimentResults, LossModel};
use super::statement::MINPACKETSIZE;

use ::byteorder::{ByteOrder, BE};

use crate::experiment::SmallishDuration;

#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize)]
pub struct Info {
    pub seqn: u32,
    pub st_us: u32,
    pub rt_us: u32,
}

pub struct PacketReceiver {
    v: Vec<Info>,
    start: Instant,
    session_id: u64,
    ctr: usize,
    cur_del_us: f64,
}

pub struct PacketReceiverParams {
    pub num_packets: u32,
    pub session_id: u64,
    pub experiment_start: Instant,
}

impl PacketReceiver {
    pub fn recv(&mut self, pkt: &[u8]) {
        assert!(pkt.len() >= MINPACKETSIZE);
        if self.ctr >= self.v.len() {
            return;
        }
        let seqn = BE::read_u32(&pkt[12..16]);
        let st_us = BE::read_u32(&pkt[16..20]);

        let recv_ts = Instant::now();
        if self.start > recv_ts {
            self.start = recv_ts
        }
        let rt_us = (recv_ts - self.start).as_us();

        self.v[self.ctr] = Info { rt_us, st_us, seqn };
        self.ctr += 1;

        self.cur_del_us = 0.8 * self.cur_del_us + 0.2 * (rt_us as f64 - st_us as f64);
    }

    pub fn current_delay(&self) -> Duration {
        Duration::from_micros(self.cur_del_us as u64)
    }

    /// for a while, no packet was received at all
    pub fn no_packet_received(&mut self) {
        /*if self.cur_del_us > 1000_000.0 {
            self.cur_del_us = 1000_000.0;
        }*/
    }

    pub fn last_sqn(&self) -> u32 {
        if self.ctr == 0 {
            0
        } else {
            self.v[self.ctr - 1].seqn
        }
    }

    pub fn new(prp: PacketReceiverParams) -> Self {
        PacketReceiver {
            start: prp.experiment_start,
            // not just with_capacity to avoid page faults while filling it in
            v: vec![Default::default(); prp.num_packets as usize],
            session_id: prp.session_id,
            ctr: 0,
            cur_del_us: 0.0,
        }
    }

    pub fn analyse(&self) -> ExperimentResults {
        let mut r = super::analyser::analyse(&self.v[0..self.ctr], self.v.len());
        r.session_id = self.session_id;
        r
    }

    #[allow(unused_parens)]
    pub fn save_raw_data(&self, dir: &::std::path::Path) {
        if let Err(e) = (try {
            let p = dir.join(format!("{}.dat", self.session_id));
            let f = ::std::fs::File::create(p)?;
            let mut f = ::std::io::BufWriter::new(f);

            ::bincode::serialize_into(&mut f, &self.v.len())?;
            ::bincode::serialize_into(f, &self.v[0..self.ctr])?;
        }) {
            let e: ::anyhow::Error = e;
            eprintln!("Error saving raw receive data: {}", e);
        }
    }

    pub fn dump_raw_data(p: &::std::path::Path) -> crate::Result<()> {
        let mut f = ::std::io::BufReader::new(::std::fs::File::open(p)?);
        let _totpkt: usize = ::bincode::deserialize_from(&mut f)?;
        let v: Vec<Info> = ::bincode::deserialize_from(f)?;
        for inf in v {
            println!(
                "{} {} {}",
                inf.seqn,
                inf.st_us as f64 / 1000.0,
                inf.rt_us as f64 / 1000.0
            );
        }
        Ok(())
    }
}
