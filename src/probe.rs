use ::structopt::StructOpt;
use crate::Result;
use ::std::net::{SocketAddr,UdpSocket,SocketAddrV4,SocketAddrV6,Ipv4Addr,Ipv6Addr};
use ::std::time::{Duration,Instant};
use crate::experiment::SmallishDuration;
use crate::experiment::statement::{ExperimentInfo,ExperimentReply,ExperimentDirection};

#[derive(Debug, StructOpt)]
pub struct Cmd {
    #[structopt(flatten)]
    pub experiment: crate::experiment::statement::ExperimentInfo,

    /// Remote UDP port to use as netmeasure2 server
    pub server: SocketAddr,

    /// Use IPv6
    #[structopt(short="6")]
    pub ipv6: bool,

    #[structopt(long="source-port", default_value="0")]
    pub source_port: u16,
}

pub fn probe(cmd:Cmd) -> Result<()> {
    let udp = UdpSocket::bind(if cmd.ipv6 {
        SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, cmd.source_port, 0, 0))
    } else {
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, cmd.source_port))
    })?;
    udp.connect(cmd.server)?;
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
        udp.send(::serde_cbor::ser::to_vec_sd(&c2s)?.as_slice())?;
        match udp.recv(&mut buf) {
            Ok(ret) => {
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
                    ExperimentReply::HereAreResults(_) => bail!("Results not expected now"),
                    ExperimentReply::RetryWithASessionId{session_id} => {
                        c2s.experiment.session_id = Some(session_id);
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

    Ok(())
}