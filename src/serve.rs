use ::structopt::StructOpt;
use ::std::net::SocketAddr;
use crate::Result;
use ::std::str::from_utf8;
use ::failure::Error;
use ::std::convert::identity;

use ::std::net::UdpSocket;

extern crate miniserde;
use ::miniserde::json::{to_string,from_str};
use ::serde_cbor::{ser::to_vec_sd,de::from_slice};

use crate::experiment::statement::{ExperimentInfo,ExperimentReply};
use crate::experiment::results::ExperimentResults;

use ::rand::Rng;

use ::std::rc::Rc;
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

struct CompletedExperiment(ExperimentInfo, Rc<ExperimentResults>);

struct OngoingExperiment {
    start_time : Instant,
    info: ExperimentInfo,
    cla : SocketAddr,
}

#[derive(Eq,PartialEq)]
struct PendingExperiment {
    cla: SocketAddr,
    sid: u64,
}

enum State {
    Idle(Option<CompletedExperiment>, Option<PendingExperiment>),
    ExperimentIsOngoing(OngoingExperiment),
}



pub fn serve(cmd:Cmd) -> Result<()> {
    let mut udp = UdpSocket::bind(cmd.sa)?;
    println!("Listening {}", cmd.sa);
    let mut buf = [0; 4096];
    let mut st = State::Idle(None, None);
    let mut rnd = ::rand::EntropyRng::new();

    loop {
        match (try {
            let (ret,cla) = udp.recv_from(&mut buf)?;
            let msg = &buf[0..ret];
            match &mut st {
                State::Idle(ref laste,ref mut pending) => {
                    let rq : ExperimentInfo = from_slice(msg)?;

                    let rp;
                    if laste.is_some() && laste.as_ref().unwrap().0 == rq {
                        rp = ExperimentReply::HereAreResults(laste.as_ref().unwrap().1.clone());
                    } else
                    if !rq.within_limits(&cmd) {
                        rp = ExperimentReply::ResourceLimits;
                    } else if rq.session_id.is_some() && pending == &Some(PendingExperiment{cla,sid:rq.session_id.unwrap()}) {
                        println!("Starting experiment: {:?}", rq);
                        udp.set_read_timeout(Some(Duration::from_secs(3)))?;

                        rp = ExperimentReply::Accepted{session_id:rq.session_id.unwrap()};
                        
                        let oe = OngoingExperiment {
                            cla,
                            info: rq,
                            start_time:  Instant::now(),
                        };
                        st = State::ExperimentIsOngoing(oe);
                    } else {
                        if pending.is_none() || pending.as_ref().unwrap().cla != cla {
                            *pending = Some(PendingExperiment{cla, sid:rnd.gen()});
                        }
                        rp = ExperimentReply::RetryWithASessionId{session_id: pending.as_ref().unwrap().sid}
                    }
                    udp.reply(&rp, cla)?;
                },
                State::ExperimentIsOngoing(oe) => {
                    if oe.cla != cla {
                        let rp = ExperimentReply::Busy;
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
        self.send_to(&to_vec_sd(rp)?[..], cla)?;
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