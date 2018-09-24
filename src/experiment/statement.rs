extern crate structopt;

use std::net::SocketAddr;
use ::structopt::StructOpt;
use ::std::rc::Rc;

use ::std::time::Duration;

use ::structopt::clap::Arg;

#[derive(Debug, EnumString, Display, Serialize, Deserialize, Eq, PartialEq, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentDirection {
    #[strum(serialize = "send")]
    ToServerOnly,

    #[strum(serialize = "recv")]
    FromServerOnly,

    #[strum(serialize = "both")]
    Bidirectional,
}

impl ExperimentDirection {
    pub fn server_needs_sender(&self) -> bool {
        match(self) { ExperimentDirection::ToServerOnly => false, _ => true, }
    }
    pub fn client_needs_sender(&self) -> bool {
        match(self) { ExperimentDirection::FromServerOnly => false, _ => true, }
    }
    pub fn server_needs_receiver(&self) -> bool { self.client_needs_sender() }
    pub fn client_needs_receiver(&self) -> bool { self.server_needs_sender() }
}


#[derive(Debug, StructOpt, Clone)]
#[derive(Serialize,Deserialize,Derivative)]
#[derivative(PartialEq)]
pub struct ExperimentInfo {
    /// Packet size for experiment, in bytes
    #[structopt(long = "packetsize", default_value = "120")]
    pub packetsize: u32,

    /// Delay between sending packets, microseconds
    #[structopt(long = "packetdelay", default_value = "10000")]
    pub packetdelay_us: u64,

    /// Total number of packets to be sent
    #[structopt(long = "totalpackets", default_value = "1000")]
    pub totalpackets: u32,

    /// Direction: send | recv | both
    #[structopt(long = "direction", default_value = "both")]
    pub direction: ExperimentDirection,

    /// Make packets looks like RTP
    #[structopt(long = "rtpmimic")]
    pub rtpmimic: bool,

    /// Internal parameter, no need to set
    #[structopt(long = "sessionid")]
    pub session_id: Option<u64>,

    /// In microseconds
    #[structopt(long = "warmup_time", default_value = "2000000")]
    #[derivative(PartialEq="ignore")]
    pub pending_start_in_microseconds: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ExperimentReply {
    /// Experiment is accepted by server
    Accepted{session_id:u64, remaining_warmup_time_us:u32},
    /// Experiment is already running, but not completed yet. Retry later to get results.
    IsOngoing{session_id:u64, elapsed_time_us:u32},
    /// Server is busy with another experiment
    Busy,
    /// Experiment is denied because of parameters are too aggressive
    ResourceLimits{msg:String},
    /// Server requests client to re-send the request with a supplied key attached
    /// (to deter spoofed source addresses DoS amplification)
    RetryWithASessionId{session_id:u64},
    /// Results are already vailable. None = receiving at server side was not requested
    HereAreResults{stats:Option<Rc<super::results::ExperimentResults>>, send_lost:Option<u32>},
    /// There was some failure on server
    Failed{msg:String},
}