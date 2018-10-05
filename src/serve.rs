use ::structopt::StructOpt;
use ::std::net::SocketAddr;
use crate::Result;
use ::std::str::from_utf8;
use ::failure::Error;
use ::std::convert::identity;

use ::std::net::UdpSocket;

use ::serde_cbor::{ser::to_vec_sd,de::from_slice};

use crate::experiment::statement::{ExperimentInfo,ExperimentReply,MINPACKETSIZE};
use crate::experiment::results::ExperimentResults;
use crate::experiment::receiver::{PacketReceiver,PacketReceiverParams};

use ::rand::Rng;

use ::std::rc::Rc;
use ::std::time::{Duration,Instant};

use crate::experiment::SmallishDuration;

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

    /// Minimum time between packets
    #[structopt(long = "min-packetdelay-us", default_value = "200")]
    min_packetdelay_us: u64,

    #[structopt(long="save-raw-stats",short="R",parse(from_os_str))]
    save_raw_stats: Option<::std::path::PathBuf>,
}

struct CompletedExperiment{
    info: ExperimentInfo,
    rcv: Option<Rc<ExperimentResults>>,
    snd: Option<u32>,
}

struct OngoingExperiment {
    start_time : Instant,
    stop_time: Instant,
    info: ExperimentInfo,
    cla : SocketAddr,
    rcv: Option<PacketReceiver>,
    snd: Option<::std::thread::JoinHandle<crate::Result<u32>>>,
}

impl OngoingExperiment {
    fn expired(&self) -> bool {
        Instant::now() > self.stop_time
    }
    fn expired2(&self) -> bool {
        if let Some(ref rcv) = self.rcv {
            let remaining = self.info.totalpackets as i64 - rcv.last_sqn() as i64;

            if remaining > 4 {
                Instant::now() > self.stop_time + Duration::from_secs(10)
            } else {
                Instant::now() > self.stop_time + Duration::from_secs(1)
            }
        } else {
            Instant::now() > self.stop_time + Duration::from_secs(1)
        }
    }
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
    fn complete_experiment(&mut self, udp: &mut UdpSocket, cmd:&Cmd) -> Result<()> {
        match self {
            State::ExperimentIsOngoing(oe) => {
                println!("Experiment completed");

                let mut ce;
                if let Some(ref mut rcv) = oe.rcv {
                    if let Some(srs) = cmd.save_raw_stats.as_ref() {
                        rcv.save_raw_data(srs);
                    }
                    ce = CompletedExperiment{
                        info: oe.info.clone(),
                        rcv: Some(Rc::new(rcv.analyse())),
                        snd: None,
                    };
                } else {
                    ce = CompletedExperiment{
                        info: oe.info.clone(),
                        rcv: None,
                        snd: None,
                    };
                }

                if let Some(snd) = oe.snd.take() {
                    match snd.join() {
                        Err(_e) => { bail!("sender thread panicked"); },
                        Ok(x) => {
                            let lost = x?;
                            ce.snd = Some(lost);
                        },
                    }
                };

                *self = State::Idle(Some(ce), None);
                udp.set_read_timeout(None)?;
            },
            State::Idle(_,_) => {
                panic!();
            }
        };
        Ok(())
    }

    fn start_experiment(&mut self, cla: SocketAddr, udp: &mut UdpSocket, rq: ExperimentInfo) -> Result<&mut OngoingExperiment> {
        match self {
            State::ExperimentIsOngoing(_oe) => {
                panic!();
            },
            State::Idle(_,_) => {
                println!("Starting experiment: {:?}", rq);
                udp.set_read_timeout(Some(Duration::from_secs(2)))?;
                
                let experiment_start = Instant::now() + Duration::from_micros(
                    rq.pending_start_in_microseconds as u64
                );
                let experiment_stop = experiment_start + rq.duration();

                let snd = if rq.direction.server_needs_sender() {
                    let sender = crate::experiment::sender::Sender {
                        delay_between_packets: Duration::from_micros(rq.packetdelay_us),
                        packetsize: rq.packetsize as usize,
                        rtpmimic: rq.rtpmimic,
                        packetcount: rq.totalpackets,
                        experiment_start,
                        session_id: rq.session_id,
                    };
                    let udp2 = udp.try_clone()?;
                    Some(::std::thread::spawn(move || {
                        sender.run(udp2, cla)
                    }))
                } else { None };

                let rcv = if rq.direction.server_needs_receiver() {
                    let prp = PacketReceiverParams {
                        experiment_start,
                        session_id: rq.session_id,
                        num_packets: rq.totalpackets,
                    };
                    Some(PacketReceiver::new(prp))
                } else { None };

                let oe = OngoingExperiment {
                    cla,
                    info: rq,
                    start_time:  experiment_start,
                    stop_time: experiment_stop,
                    rcv,
                    snd,
                };
                *self = State::ExperimentIsOngoing(oe);
            }
        };
        match self {
            State::ExperimentIsOngoing(oe) => {
                Ok(oe)
            },
            State::Idle(_,_) => {
                unreachable!()
            },
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
        let mut prev_cla = None;
        match (try {
            let (ret,cla) = match udp.recv_from(&mut buf) {
                Ok(x) => x,
                Err(ref e) if e.kind() == ::std::io::ErrorKind::WouldBlock => {
                    match st {
                        State::ExperimentIsOngoing(ref oe) => {
                            if oe.expired() {
                                st.complete_experiment(&mut udp,&cmd)?;
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
            prev_cla = Some(cla);
            let msg = &buf[0..ret];
            match &mut st {
                State::Idle(ref laste,ref mut pending) => {
                    if msg.len() < MINPACKETSIZE {
                        continue;
                    }
                    if &msg[0..3] != b"\xd9\xd9\xf7" {
                        continue;
                    }
                    let s2c : super::ClientToServer = from_slice(msg)?;
                    if s2c.api_version != crate::API_VERSION {
                        println!("Invalid API version");
                        continue;
                    }
                    let rq : ExperimentInfo = s2c.experiment;
                    let seqn_for_rtt = s2c.seqn_for_rtt;

                    let rp;
                    if laste.is_some() && laste.as_ref().unwrap().info == rq {
                        rp = ExperimentReply::HereAreResults{
                            stats: laste.as_ref().unwrap().rcv.clone(),
                            send_lost: laste.as_ref().unwrap().snd,
                        };
                    } else
                    if let Err(e) = rq.check_limits(&cmd) {
                        rp = ExperimentReply::ResourceLimits{msg:e.to_string()};
                    } else if pending == &Some(PendingExperiment{cla,sid:rq.session_id}) {
                        let oe = st.start_experiment(cla,&mut udp,rq)?;
                        let mut remaining_warmup_time_us = 0;
                        let now = Instant::now();
                        if oe.start_time > now {
                            remaining_warmup_time_us = (oe.start_time-now).as_us();
                        }
                        rp = ExperimentReply::Accepted{
                            session_id: oe.info.session_id,
                            remaining_warmup_time_us,
                        };
                    } else {
                        if pending.is_none() || pending.as_ref().unwrap().cla != cla {
                            *pending = Some(PendingExperiment{cla, sid:rnd.gen()});
                        }
                        rp = ExperimentReply::RetryWithASessionId{session_id: pending.as_ref().unwrap().sid}
                    }
                    udp.reply(rp, cla, seqn_for_rtt)?;
                },
                State::ExperimentIsOngoing(oe) => {
                    if oe.cla != cla {
                        let rp = ExperimentReply::Busy;
                        udp.reply(rp, cla, 0)?;
                        continue;
                    }
                    
                    if msg.len() < MINPACKETSIZE {
                        // dwarf packet
                        continue;
                    }

                    if &msg[0..3] == b"\xd9\xd9\xf7" {
                        let s2c : super::ClientToServer = from_slice(msg)?;
                        if s2c.api_version != crate::API_VERSION {
                            bail!("Invalid API version");
                        }
                        let rq : ExperimentInfo = s2c.experiment;
                        let seqn_for_rtt = s2c.seqn_for_rtt;

                        let rp;
                        if rq == oe.info {
                            let n = Instant::now();
                            if n >= oe.start_time {
                                let elapsed_time_us = (n - oe.start_time).as_us();
                                rp = ExperimentReply::IsOngoing {
                                    session_id: oe.info.session_id,
                                    elapsed_time_us,
                                };
                            } else {
                                let remaining_warmup_time_us = (oe.start_time - n).as_us();
                                rp = ExperimentReply::Accepted {
                                    session_id: oe.info.session_id,
                                    remaining_warmup_time_us,
                                };
                            }
                        } else {
                            eprintln!("{:?}", rq);
                            eprintln!("!=");
                            eprintln!("{:?}", oe.info);

                            rp = ExperimentReply::Busy;
                        }
                        udp.reply(rp, cla, seqn_for_rtt)?;

                        if oe.expired2() {
                            st.complete_experiment(&mut udp, &cmd)?;
                        }
                        continue;
                    }
                    
                    if &msg[0..3] == b"\x00\x00\x00" {
                        if let Some(ref mut rcv) = oe.rcv {
                            rcv.recv(msg);
                        }
                        continue;
                    }

                    if &msg[0..2] == b"\x80\x64" {
                        // RTP mode
                        if let Some(ref mut rcv) = oe.rcv {
                            rcv.recv(msg);
                        }
                        continue;
                    }

                    println!("Unknown packet beginning with {:?}", &msg[0..3]);
                },
            };
        }) {
            Ok(()) => (),
            Err(e) => {
                println!("error: {} {:?}", identity::<&Error>(&e), &e);
                if let Some(cla) = prev_cla {
                    let _  = udp.reply(ExperimentReply::Failed{msg:format!("{}", e)}, cla, 0);
                }
            },
        }
    }
}

trait ExperimentNegotiation {
    fn reply(&mut self, rp: ExperimentReply, cla: SocketAddr, seqn_for_rtt: u32) -> Result<()>;
}

impl ExperimentNegotiation for UdpSocket {
    fn reply(&mut self, rp: ExperimentReply, cla: SocketAddr, seqn_for_rtt: u32) -> Result<()> {
        let s2c = crate::ServerToClient::from((rp,seqn_for_rtt));
        self.send_to(&to_vec_sd(&s2c)?[..], cla)?;
        Ok(())
    }
}

impl ExperimentInfo {
    pub fn check_limits(&self, cmd: &Cmd) -> ::std::result::Result<(),&'static str> {
        if self.packetdelay_us == 0 { return Err("zero packet delay") }
        if self.packetdelay_us < cmd.min_packetdelay_us {
            return Err("packetdelay too low");
        }
        let bw_kbps = self.kbps();
        let maxdur = Duration::from_secs(cmd.timelimit.into());

        if self.totalpackets > 1_000_0000 { return Err("total packets too big") }
        if self.packetdelay_us > 60_000_000 { return Err("packet delay too big") }

        if self.packetsize < MINPACKETSIZE as u32 || self.packetsize > 10000 {
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

    pub fn kbps(&self) -> u32 {
        let effective_ps = (self.packetsize+32).max(64);
        let pps = 1000_000.0 / (self.packetdelay_us as f64);
        ((effective_ps as f64) * pps * 8.0 / 1000.0) as u32
    }

    pub fn bytes_used(&self) -> u32 {
        self.totalpackets * (self.packetsize+24)
    }
}