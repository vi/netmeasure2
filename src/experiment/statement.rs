extern crate structopt;
extern crate miniserde;
extern crate miniserde_tools;

use std::net::SocketAddr;
use ::structopt::StructOpt;


#[derive(Debug, EnumString, Display)]
pub enum ExperimentDirection {
    #[strum(serialize = "send")]
    ToServerOnly,

    #[strum(serialize = "recv")]
    FromServerOnly,

    #[strum(serialize = "both")]
    Bidirectional,
}

miniserialize_for_display!(ExperimentDirection);
minideserialize_for_fromstr!(ExperimentDirection);


#[derive(Debug, StructOpt)]
#[derive(MiniSerialize,MiniDeserialize)]
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

    #[structopt(long = "rtpmimic")]
    pub rtpmimic: bool,

    #[structopt(long = "sesionid")]
    pub session_id: Option<u64>,
}

#[derive(Debug, EnumString, Display)]
pub enum ExperimentReplyCode {
    /// Experiment is accepted by server
    Accepted,
    /// Server is busy with another experiment
    Busy,
    /// Experiment is denied because of parameters are too aggressive
    ResourceLimits,
    /// Server requests client to re-send the request with a supplied key attached
    /// (to deter spoofed source addresses)
    RetryWithASessionId,
    /// Invalid request, etc.
    SomeOtherError,
}

miniserialize_for_display!(ExperimentReplyCode);
minideserialize_for_fromstr!(ExperimentReplyCode);

#[derive(Debug, MiniDeserialize, MiniSerialize)]
pub struct ExperimentReply {
    pub status: ExperimentReplyCode,
    pub session_id: u64,
}