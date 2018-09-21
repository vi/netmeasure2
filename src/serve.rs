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
use crate::experiment::receiver::{PacketReceiver,PacketReceiverParams};

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
    rcv: PacketReceiver,
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

impl State {
    fn complete_experiment(&mut self, udp: &mut UdpSocket) {
        match self {
            State::ExperimentIsOngoing(oe) => {
                println!("Experiment completed");
                let ce = CompletedExperiment(oe.info.clone(), Rc::new(oe.rcv.analyse()));
                *self = State::Idle(Some(ce), None);
                let _ = udp.set_read_timeout(None);
            },
            State::Idle(_,_) => {
                panic!();
            }
        }
    }
}


pub fn serve(cmd:Cmd) -> Result<()> {
    let mut udp = UdpSocket::bind(cmd.sa)?;
    println!("Listening {}", cmd.sa);
    let mut buf = [0; 4096];
    let mut st = State::Idle(None, None);
    let mut rnd = ::rand::EntropyRng::new();

    loop {
        match (try {
            let (ret,cla) = match udp.recv_from(&mut buf) {
                Ok(x) => x,
                Err(ref e) if e.kind() == ::std::io::ErrorKind::WouldBlock => {
                    match st {
                        State::ExperimentIsOngoing(ref oe) => {
                            if oe.rcv.expired() {
                                st.complete_experiment(&mut udp);
                            };
                        },
                        _ => {
                            println!("unexpected wouldblock");
                        },
                    };
                    continue;
                },
                Err(e) => Err(e)?,
            };
            let msg = &buf[0..ret];
            match &mut st {
                State::Idle(ref laste,ref mut pending) => {
                    let s2c : super::ClientToServer = from_slice(msg)?;
                    if s2c.api_version != crate::API_VERSION {
                        println!("Invalid API version");
                        continue;
                    }
                    let rq : ExperimentInfo = s2c.experiment;

                    let rp;
                    if laste.is_some() && laste.as_ref().unwrap().0 == rq {
                        rp = ExperimentReply::HereAreResults(laste.as_ref().unwrap().1.clone());
                    } else
                    if let Err(e) = rq.check_limits(&cmd) {
                        rp = ExperimentReply::ResourceLimits{msg:e.to_string()};
                    } else if rq.session_id.is_some() && pending == &Some(PendingExperiment{cla,sid:rq.session_id.unwrap()}) {
                        println!("Starting experiment: {:?}", rq);
                        udp.set_read_timeout(Some(Duration::from_secs(2)))?;

                        rp = ExperimentReply::Accepted{session_id:rq.session_id.unwrap()};
                        
                        let experiment_start = Instant::now() + Duration::from_micros(
                            rq.pending_start_in_microseconds as u64
                        );
                        let experiment_stop = experiment_start + rq.duration();

                        let prp = PacketReceiverParams {
                            experiment_start,
                            experiment_stop,
                            session_id: rq.session_id.unwrap(),
                            num_packets: rq.totalpackets,
                        };

                        let oe = OngoingExperiment {
                            cla,
                            info: rq,
                            start_time:  Instant::now(),
                            rcv: PacketReceiver::new(prp),
                        };
                        st = State::ExperimentIsOngoing(oe);
                    } else {
                        if pending.is_none() || pending.as_ref().unwrap().cla != cla {
                            *pending = Some(PendingExperiment{cla, sid:rnd.gen()});
                        }
                        rp = ExperimentReply::RetryWithASessionId{session_id: pending.as_ref().unwrap().sid}
                    }
                    udp.reply(rp, cla)?;
                },
                State::ExperimentIsOngoing(oe) => {
                    if oe.cla != cla {
                        let rp = ExperimentReply::Busy;
                        udp.reply(rp, cla)?;
                        continue;
                    }
                    
                    if msg.len() < 16 {
                        // dwarf packet
                        continue;
                    }

                    if &msg[0..3] == b"\xd9\xd9\xf7" {
                        let rq : ExperimentInfo = from_slice(msg)?;
                        let rp;
                        if rq == oe.info {
                            rp = ExperimentReply::IsOngoing;
                        } else {
                            eprintln!("{:?}", rq);
                            eprintln!("!=");
                            eprintln!("{:?}", oe.info);

                            rp = ExperimentReply::Busy;
                        }
                        udp.reply(rp, cla)?;

                        if oe.rcv.expired() {
                            st.complete_experiment(&mut udp);
                        }
                        continue;
                    }
                    
                    if &msg[0..3] == b"\x00\x00\x00" {
                        oe.rcv.recv(msg);
                        continue;
                    }

                    if &msg[0..3] == b"" {
                        // TODO: RTP mode
                    }

                    println!("Unknown packet beginning with {:?}", &msg[0..3]);
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
    fn reply(&mut self, rp: ExperimentReply, cla: SocketAddr) -> Result<()>;
}

impl ExperimentNegotiation for UdpSocket {
    fn reply(&mut self, rp: ExperimentReply, cla: SocketAddr) -> Result<()> {
        let s2c = crate::ServerToClient::from(rp);
        self.send_to(&to_vec_sd(&s2c)?[..], cla)?;
        Ok(())
    }
}

impl ExperimentInfo {
    pub fn check_limits(&self, cmd: &Cmd) -> ::std::result::Result<(),&'static str> {
        let effective_ps = (self.packetsize+32).max(64);
        if self.packetdelay_us == 0 { return Err("zero packet delay") }
        let pps = 1000_000.0 / (self.packetdelay_us as f64);
        let bw_kbps = ((effective_ps as f64) * pps * 8.0 / 1024.0) as u32;
        let maxdur = Duration::from_secs(cmd.timelimit.into());

        if self.totalpackets > 1_000_0000 { return Err("total packets too big") }
        if self.packetdelay_us > 60_000_000 { return Err("packet delay too big") }

        if self.packetsize < 16 || self.packetsize > 10000 {
            return Err("invalid packetsize");
        }

        if self.pending_start_in_microseconds > 5_000_000 { return Err("pending start too late") }

        if self.duration() > maxdur {return Err("duration too long") }
        if bw_kbps > cmd.bwlimit { return Err("bwlimit") }
        Ok(())
    }

    pub fn duration(&self) -> Duration {
        Duration::from_micros(self.packetdelay_us) * self.totalpackets
    }
}