use ::structopt::StructOpt;
use ::std::net::SocketAddr;
use ::std::io::Result;
use ::std::str::from_utf8;
use ::failure::Error;
use ::std::convert::identity;

use ::std::net::UdpSocket;

extern crate miniserde;
use ::miniserde::json::{to_string,from_str};

use crate::experiment::statement::{ExperimentInfo,ExperimentReply,ExperimentReplyCode};

use ::rand::Rng;

use ::std::time::{Duration,Instant};

#[derive(Debug, StructOpt)]
pub struct Cmd {
    /// UDP port to listen
    sa: SocketAddr,

    /// One experiment time limit, seconds
    #[structopt(long = "timelimit", default_value = "30")]
    timelimit: u32,

    /// One experiment bandwidth limit, kilobits per second
    #[structopt(long = "bwlimit", default_value = "50000")]
    bwlimit: u32,
}

enum State {
    Idle,
    ExperimentIsOngoing(ExperimentInfo,SocketAddr,Instant),
    //DeliveringResults,
}



pub fn serve(cmd:Cmd) -> Result<()> {
    let mut udp = UdpSocket::bind(cmd.sa)?;
    println!("Listening {}", cmd.sa);
    let mut buf = [0; 4096];
    let mut st = State::Idle;
    let mut rnd = ::rand::EntropyRng::new();
    let mut cur_sid = None;
    loop {
        match (try {
            let (ret,cla) = udp.recv_from(&mut buf)?;
            let msg = &buf[0..ret];
            match &st {
                State::Idle => {
                    let rq : ExperimentInfo = from_str(from_utf8(msg)?)?;

                    let rp;
                    if !rq.within_limits(&cmd) {
                        rp = ExperimentReply {
                            session_id:0,
                            status:ExperimentReplyCode::ResourceLimits,
                        };
                    } else if cur_sid.is_some() && rq.session_id == cur_sid {
                        println!("Starting experiment: {:?}", rq);
                        udp.set_read_timeout(Some(Duration::from_secs(3)))?;

                        let session_id = cur_sid.unwrap();
                        rp = ExperimentReply {session_id, status:ExperimentReplyCode::Accepted};
                        
                        cur_sid = None;
                        st = State::ExperimentIsOngoing(rq, cla, Instant::now());
                    } else {
                        if cur_sid.is_none() {
                            cur_sid = Some(rnd.gen());
                        }
                        let session_id = cur_sid.unwrap();
                        rp = ExperimentReply {session_id, status:ExperimentReplyCode::RetryWithASessionId};
                    }
                    udp.reply(&rp, cla)?;
                },
                State::ExperimentIsOngoing(e,cla_,startt) => {
                    if cla_ != &cla {
                        let rp = ExperimentReply {session_id:0, status:ExperimentReplyCode::Busy};
                        udp.reply(&rp, cla)?;
                        continue;
                    }
                },
            };
        }) {
            Ok(()) => (),
            Err(e) => {
                println!("error: {} {:?}", identity::<&Error>(&e), &e);
            },
        }
    }
}

trait ExperimentNegotiation {
    fn reply(&mut self, rp: &ExperimentReply, cla: SocketAddr) -> Result<()>;
}

impl ExperimentNegotiation for UdpSocket {
    fn reply(&mut self, rp: &ExperimentReply, cla: SocketAddr) -> Result<()> {
        self.send_to(to_string(rp).as_bytes(), cla)?;
        Ok(())
    }
}

impl ExperimentInfo {
    pub fn within_limits(&self, cmd: &Cmd) -> bool {
        let effective_ps = (self.packetsize+32).max(64);
        if self.packetdelay_us == 0 { return false }
        let pps = 1000_000.0 / (self.packetdelay_us as f64);
        let bw_kbps = ((effective_ps as f64) * pps * 8.0 / 1024.0) as u32;
        let maxdur = Duration::from_secs(cmd.timelimit.into());

        if self.totalpackets > 1_000_0000 { return false }
        if self.packetdelay_us > 60_000_000 { return false }

        self.duration() <= maxdur && bw_kbps <= cmd.bwlimit
    }

    pub fn duration(&self) -> Duration {
        Duration::from_micros(self.packetdelay_us) * self.totalpackets
    }
}