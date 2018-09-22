use ::structopt::StructOpt;
use crate::Result;
use ::std::net::{SocketAddr,UdpSocket,SocketAddrV4,SocketAddrV6,Ipv4Addr,Ipv6Addr};
use ::std::time::{Duration,Instant};
use crate::experiment::SmallishDuration;
use crate::experiment::statement::{ExperimentInfo,ExperimentReply,ExperimentDirection};
use crate::experiment::results::ExperimentResults;
use ::std::rc::Rc;

#[derive(Debug, StructOpt)]
pub struct Cmd {
    #[structopt(flatten)]
    pub experiment: ExperimentInfo,

    /// Remote UDP port to use as netmeasure2 server
    pub server: SocketAddr,

    /// Use IPv6
    #[structopt(short="6")]
    pub ipv6: bool,

    #[structopt(long="source-port", default_value="0")]
    pub source_port: u16,

    #[structopt(long="output", short="o", parse(from_os_str))]
    pub output: Option<::std::path::PathBuf>,

    #[structopt(long="save-raw-stats",short="R",parse(from_os_str))]
    save_raw_stats: Option<::std::path::PathBuf>,
}

pub fn probe(cmd:Cmd) -> Result<()> {
    let udp = UdpSocket::bind(if cmd.ipv6 {
        SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, cmd.source_port, 0, 0))
    } else {
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, cmd.source_port))
    })?;
    udp.set_read_timeout(Some(Duration::from_millis(250)))?;

    let mut c2s = crate::ClientToServer {
        experiment: cmd.experiment,
        api_version: crate::API_VERSION,
    };

    let mut buf = [0; 1536];

    let s2c : crate::ServerToClient;

    let start = Instant::now() + Duration::from_micros(c2s.experiment.pending_start_in_microseconds as u64);
    let end = start + c2s.experiment.duration();

    eprint!("Sending request");
    loop {
        let now = Instant::now();
        if now > start {
            eprintln!(" timeout");
            bail!("timeout");
        }
        let ttg = (start - now);
        c2s.experiment.pending_start_in_microseconds = ttg.as_us();
        eprint!(".");
        //eprintln!("{:?}", c2s);
        udp.send_to(::serde_cbor::ser::to_vec_sd(&c2s)?.as_slice(), cmd.server)?;
        match udp.recv_from(&mut buf) {
            Ok((ret,from)) => {
                if (from != cmd.server) {
                    eprintln!("Foreign packet");
                    continue;
                }

                let s2c : crate::ServerToClient = ::serde_cbor::from_slice(&buf[0..ret])?;

                if s2c.api_version != crate::API_VERSION {
                    bail!("Wrong API version");
                }

                match s2c.reply {
                    ExperimentReply::Busy => bail!("Server busy"),
                    ExperimentReply::Accepted{session_id} => {
                        assert!(Some(session_id) == c2s.experiment.session_id);
                        break;
                    },
                    ExperimentReply::ResourceLimits{msg} => {
                        eprintln!("\nResource limits: {}", msg);
                        bail!("Parameters out of range");
                    },
                    ExperimentReply::IsOngoing => break,
                    ExperimentReply::HereAreResults{stats:_} => bail!("Results not expected now"),
                    ExperimentReply::RetryWithASessionId{session_id} => {
                        c2s.experiment.session_id = Some(session_id);
                    },
                    ExperimentReply::Failed{msg} => {
                        eprintln!("{}",msg);
                        bail!("Fail reply from server");
                    },
                };
            },
            Err(ref e) if e.kind() == ::std::io::ErrorKind::WouldBlock => {
                continue;
            },
            Err(e) => Err(e)?,
        }
    }
    eprintln!();
    eprintln!("Experiment started");

    let mut rcv = if c2s.experiment.direction.client_needs_receiver() {
        Some(crate::experiment::receiver::PacketReceiver::new(
            crate::experiment::receiver::PacketReceiverParams {
                num_packets: c2s.experiment.totalpackets,
                session_id: c2s.experiment.session_id.unwrap(),
                experiment_start: start,
            }
        ))
    } else { None };

    let snd = if c2s.experiment.direction.client_needs_sender() {
        let udp2 = udp.try_clone()?;
        let serv2 = cmd.server;
        let sender = crate::experiment::sender::Sender {
            delay_between_packets: Duration::from_micros(c2s.experiment.packetdelay_us),
            packetcount: c2s.experiment.totalpackets,
            packetsize: c2s.experiment.packetsize as usize,
            rtpmimic: c2s.experiment.rtpmimic,
            experiment_start: start,
        };
        Some(::std::thread::spawn(move || {
            if let Err(e) = sender.run(udp2, serv2) {
                eprintln!("Sender thread: {}", e);
                return Err(e);
            };
            Ok(())
        }))
    } else { None };

    udp.set_read_timeout(Some(Duration::from_secs(1)))?;

    let mut request_results = false;

    let results_ : Option<Rc<ExperimentResults>>;
    loop {
        let now = Instant::now();
        if !request_results && now > end {
            eprintln!("Experiment finished");
            request_results = true;

            if let Some(ref srs) = cmd.save_raw_stats {
                if let Some(ref mut rcv) = rcv {
                    rcv.save_raw_data(srs);
                }
            }
        }

        if request_results && now > end + Duration::from_secs(10) {
            bail!("Timed out waiting for results");
        }

        match udp.recv_from(&mut buf) {
            Ok((ret,from)) => {
                let msg = &buf[0..ret];

                if from != cmd.server {
                    eprintln!("foreign packet");
                    continue;
                }

                if &msg[0..3] == b"\x00\x00\x00" {
                    if let Some(ref mut rcv) = rcv {
                        rcv.recv(msg);
                    }
                    continue;
                }

                // TODO: RTP mode

                if &msg[0..3] != b"\xd9\xd9\xf7" {
                    eprintln!("Unexpected packet");
                    continue;
                }

                let s2c : crate::ServerToClient = ::serde_cbor::from_slice(msg)?;

                if s2c.api_version != crate::API_VERSION {
                    bail!("Wrong API version ; 2");
                }

                match s2c.reply {
                    ExperimentReply::Busy => bail!("Server busy 2"),
                    ExperimentReply::Accepted{session_id} => bail!("Accepted 2?"),
                    ExperimentReply::ResourceLimits{msg} => {
                        eprintln!("\nResource limits: {}", msg);
                        bail!("Parameters out of range 2");
                    },
                    ExperimentReply::IsOngoing => continue,
                    ExperimentReply::HereAreResults{stats} => {
                        if let Some(ref x) = stats {
                            ensure!(Some(x.session_id) == c2s.experiment.session_id,
                                "wrong session id in results"
                            );
                        }
                        results_ = stats;
                        break;
                    },
                    ExperimentReply::RetryWithASessionId{session_id} => bail!("Unexpected retryWSId"),
                    ExperimentReply::Failed{msg} => {
                        eprintln!("{}",msg);
                        bail!("Fail reply from server 2");
                    },
                };
            },
            Err(ref e) if e.kind() == ::std::io::ErrorKind::WouldBlock => {
                if request_results {
                    udp.send_to(::serde_cbor::ser::to_vec_sd(&c2s)?.as_slice(), cmd.server)?;
                }
            },
            Err(e) => Err(e)?,
        }
    }
    eprintln!("Results received");

    if let Some(snd) = snd {
        match snd.join() {
            Err(e) => { bail!("sender thread panicked"); },
            Ok(x) => x?,
        }
    }

    let final_result = crate::experiment::results::ResultsForStoring {
        to_server: results_,
        from_server: rcv.map(|rcv|Rc::new(rcv.analyse())),
        conditions: c2s.experiment,
    };

    let out : Box<dyn(::std::io::Write)>;
    if let Some(pb) = cmd.output {
        let mut f = ::std::fs::File::create(pb)?;
        out = Box::new(f);
    } else {
        out = Box::new(::std::io::stdout());
    }
    let mut out = ::std::io::BufWriter::new(out);
    ::serde_json::ser::to_writer(&mut out, &final_result)?;
    use ::std::io::Write;
    writeln!(out);

    Ok(())
}
