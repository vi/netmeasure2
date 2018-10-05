use ::structopt::StructOpt;
use crate::Result;
use ::std::net::{SocketAddr,UdpSocket,SocketAddrV4,SocketAddrV6,Ipv4Addr,Ipv6Addr};
use ::std::time::{Duration,Instant};
use crate::experiment::SmallishDuration;
use crate::experiment::statement::{ExperimentInfo,ExperimentReply,ExperimentDirection};
use crate::experiment::results::{ExperimentResults,ResultsForStoring};
use ::std::rc::Rc;


#[derive(Debug, StructOpt, Clone)]
pub struct CommunicOpts {
    /// Remote UDP port to use as netmeasure2 server
    pub server: SocketAddr,

    /// Use IPv6
    #[structopt(short="6")]
    pub ipv6: bool,

    #[structopt(long="source-port", default_value="0")]
    pub source_port: u16,

    #[structopt(long="save-raw-stats",short="R",parse(from_os_str))]
    save_raw_stats: Option<::std::path::PathBuf>,

    /// Maximum number of seconds to wait for results
    #[structopt(long="max-wait-for-results", default_value="15")]
    max_wait_for_results: u64,
}

#[derive(Debug, StructOpt, Clone)]
pub struct CmdImpl {
    #[structopt(flatten)]
    pub experiment: ExperimentInfo,

    #[structopt(flatten)]
    pub co : CommunicOpts,
}

#[derive(Debug, StructOpt)]
pub struct Cmd {
    #[structopt(flatten)]
    pub inner: CmdImpl,

    #[structopt(long="output", short="o", parse(from_os_str))]
    pub output: Option<::std::path::PathBuf>,

    /// Format results nicely to stdout
    /// (maybe in addition to outputing JSON to `-o` file)
    #[structopt(short="S")]
    visualise: bool,
}

pub fn probe_impl(cmd:CmdImpl) -> Result<ResultsForStoring> {
    let udp = UdpSocket::bind(if cmd.co.ipv6 {
        SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, cmd.co.source_port, 0, 0))
    } else {
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, cmd.co.source_port))
    })?;
    udp.set_read_timeout(Some(Duration::from_millis(250)))?;

    let mut c2s = crate::ClientToServer {
        experiment: cmd.experiment,
        api_version: crate::API_VERSION,
        seqn_for_rtt: 0,
    };

    let mut buf = [0; 1536];

    let _s2c : crate::ServerToClient;

    let start = Instant::now() + Duration::from_micros(c2s.experiment.pending_start_in_microseconds as u64);
    let end = start + c2s.experiment.duration() + Duration::from_secs(1);
    let mut end2 = end;

    let mut experiment_start_for_receiver = start;

    let mut ts_for_rtt_send = ::std::collections::BTreeMap::<u32, Instant>::new();
    let mut ts_for_rtt_recv = ::std::collections::BTreeMap::<u32, Instant>::new();

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
        c2s.seqn_for_rtt+=1;
        ts_for_rtt_send.insert(c2s.seqn_for_rtt, Instant::now());
        udp.send_to(::serde_cbor::ser::to_vec_sd(&c2s)?.as_slice(), cmd.co.server)?;
        match udp.recv_from(&mut buf) {
            Ok((ret,from)) => {
                if (from != cmd.co.server) {
                    eprintln!("Foreign packet");
                    continue;
                }

                let s2c : crate::ServerToClient = ::serde_cbor::from_slice(&buf[0..ret])?;

                if s2c.api_version != crate::API_VERSION {
                    bail!("Wrong API version");
                }
                ts_for_rtt_recv.insert(s2c.seqn_for_rtt, Instant::now());

                match s2c.reply {
                    ExperimentReply::Busy => bail!("Server busy"),
                    ExperimentReply::Accepted{session_id,remaining_warmup_time_us} => {
                        assert!(session_id == c2s.experiment.session_id);
                        experiment_start_for_receiver =
                            Instant::now() + Duration::from_micros(remaining_warmup_time_us as u64);
                        break;
                    },
                    ExperimentReply::IsOngoing{session_id,elapsed_time_us} => {
                        assert!(session_id == c2s.experiment.session_id);
                        experiment_start_for_receiver =
                            Instant::now() - Duration::from_micros(elapsed_time_us as u64);
                        break;
                    },
                    ExperimentReply::ResourceLimits{msg} => {
                        eprintln!("\nResource limits: {}", msg);
                        bail!("Parameters out of range");
                    },
                    ExperimentReply::HereAreResults{..} => bail!("Results not expected now"),
                    ExperimentReply::RetryWithASessionId{session_id} => {
                        c2s.experiment.session_id = session_id;
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
                session_id: c2s.experiment.session_id,
                experiment_start: experiment_start_for_receiver,
            }
        ))
    } else { None };

    let snd = if c2s.experiment.direction.client_needs_sender() {
        let udp2 = udp.try_clone()?;
        let serv2 = cmd.co.server;
        let sender = crate::experiment::sender::Sender {
            delay_between_packets: Duration::from_micros(c2s.experiment.packetdelay_us),
            packetcount: c2s.experiment.totalpackets,
            packetsize: c2s.experiment.packetsize as usize,
            rtpmimic: c2s.experiment.rtpmimic,
            experiment_start: start,
            session_id: c2s.experiment.session_id,
        };
        Some(::std::thread::spawn(move || {
            sender.run(udp2, serv2)
        }))
    } else { None };

    udp.set_read_timeout(Some(Duration::from_secs(1)))?;

    let mut request_results = false;

    let mut results_ : Option<Rc<ExperimentResults>>;
    let send_lost_: Option<u32>;

    loop {
        let now = Instant::now();
        let mut addendum = Duration::from_secs(0);
        if let Some(ref rcv) = rcv {
            let remaining = c2s.experiment.totalpackets as i64 - rcv.last_sqn() as i64;
            if remaining > 4 {
                addendum = Duration::from_secs(10);
            }
        }
        if !request_results && now > end + addendum {
            eprintln!("Experiment finished");
            request_results = true;

            if let Some(ref srs) = cmd.co.save_raw_stats {
                if let Some(ref mut rcv) = rcv {
                    rcv.save_raw_data(srs);
                }
            }
            end2 = now;
        }

        if request_results && now > end2 + Duration::from_secs(cmd.co.max_wait_for_results) {
            bail!("Timed out waiting for results");
        }

        match udp.recv_from(&mut buf) {
            Ok((ret,from)) => {
                let msg = &buf[0..ret];

                if from != cmd.co.server {
                    eprintln!("foreign packet");
                    continue;
                }

                if ret < crate::experiment::statement::MINPACKETSIZE {
                    continue;
                }

                if &msg[0..3] == b"\x00\x00\x00" {
                    if let Some(ref mut rcv) = rcv {
                        rcv.recv(msg);
                    }
                    continue;
                }

                if &msg[0..2] == b"\x80\x64" {
                    // RTP mode
                    if let Some(ref mut rcv) = rcv {
                        rcv.recv(msg);
                    }
                    continue;
                }


                if &msg[0..3] != b"\xd9\xd9\xf7" {
                    eprintln!("Unexpected packet");
                    continue;
                }

                let s2c : crate::ServerToClient = ::serde_cbor::from_slice(msg)?;

                if s2c.api_version != crate::API_VERSION {
                    bail!("Wrong API version ; 2");
                }
                ts_for_rtt_recv.insert(s2c.seqn_for_rtt, Instant::now());

                match s2c.reply {
                    ExperimentReply::Busy => bail!("Server busy 2"),
                    ExperimentReply::Accepted{session_id: _,remaining_warmup_time_us: _} => {
                        continue;  
                    },
                    ExperimentReply::IsOngoing{session_id: _,elapsed_time_us: _} => {
                        continue;
                    },
                    ExperimentReply::ResourceLimits{msg} => {
                        eprintln!("\nResource limits: {}", msg);
                        bail!("Parameters out of range 2");
                    },
                    ExperimentReply::HereAreResults{stats,send_lost} => {
                        if let Some(ref x) = stats {
                            ensure!(x.session_id == c2s.experiment.session_id,
                                "wrong session id in results"
                            );
                        }
                        send_lost_ = send_lost;
                        results_ = stats;
                        break;
                    },
                    ExperimentReply::RetryWithASessionId{session_id: _} => bail!("Unexpected retryWSId"),
                    ExperimentReply::Failed{msg} => {
                        eprintln!("{}",msg);
                        bail!("Fail reply from server 2");
                    },
                };
            },
            Err(ref e) if e.kind() == ::std::io::ErrorKind::WouldBlock => {
                if request_results {
                    c2s.seqn_for_rtt+=1;
                    ts_for_rtt_send.insert(c2s.seqn_for_rtt, Instant::now());
                    udp.send_to(::serde_cbor::ser::to_vec_sd(&c2s)?.as_slice(), cmd.co.server)?;
                }
                if let Some(ref mut rcv) = rcv {
                    eprintln!("(no packets getting received now)");
                    rcv.no_packet_received();
                }
            },
            Err(e) => Err(e)?,
        }
    }
    eprintln!("Results received");

    let mut my_send_lost = None;
    if let Some(snd) = snd {
        match snd.join() {
            Err(_e) => { bail!("sender thread panicked"); },
            Ok(x) => {
                let lost = x?;
                my_send_lost = Some(lost);
            },
        }
    }

    let mut from_server = None;
    if let Some(rcv) = rcv {
        let mut r = rcv.analyse();
        let lp = send_lost_.unwrap();
        r.loss_model.sendside_loss = lp as f32 / c2s.experiment.totalpackets as f32;
        from_server = Some(Rc::new(r));
    };
    if let Some(to_server) = results_.as_mut() {
        let lp = my_send_lost.unwrap();
        let mut r = (**to_server).clone();
        r.loss_model.sendside_loss = lp as f32 / c2s.experiment.totalpackets as f32;
        *to_server = Rc::new(r);
    }
    let rtt_us;
    {
        let mut count=0;
        let mut dur = Duration::from_secs(0);
        for (sq,ts) in ts_for_rtt_recv {
            dur += ts - ts_for_rtt_send[&sq];
            count+=1;
        }
        rtt_us = dur.as_us() / count;
    }
    let final_result = ResultsForStoring {
        to_server: results_,
        from_server,
        conditions: c2s.experiment,
        rtt_us,
        api_version: crate::API_VERSION,
    };
    Ok(final_result)
}


pub fn probe(cmd:Cmd) -> Result<()> {
    let final_result = probe_impl(cmd.inner)?;

    if cmd.visualise && cmd.output.is_none() {
        final_result.print_to_stdout();
    } else {
        let out : Box<dyn(::std::io::Write)>;
        if let Some(pb) = cmd.output {
            let f = ::std::fs::File::create(pb)?;
            out = Box::new(f);
            if cmd.visualise {
                final_result.print_to_stdout();
            }
        } else {
            out = Box::new(::std::io::stdout());
        }
        let mut out = ::std::io::BufWriter::new(out);
        ::serde_json::ser::to_writer(&mut out, &final_result)?;
        use ::std::io::Write;
        writeln!(out);
    }

    Ok(())
}
