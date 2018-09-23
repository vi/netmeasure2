#![feature(min_const_fn)]
#![feature(const_slice_len)]
#![feature(vec_resize_default)]
#![feature(try_blocks)]
#![feature(convert_id)]
#![feature(nll)]

#![allow(unused)]
#![deny(unused_must_use)]

#[macro_use]
extern crate failure;
extern crate structopt;

extern crate strum;
#[macro_use]
extern crate strum_macros;

#[macro_use]
extern crate enum_unitary;

extern crate rand;

#[macro_use]
extern crate counted_array;

#[macro_use]
extern crate static_assertions;

#[macro_use] 
extern crate serde_derive;

extern crate serde_cbor;
extern crate serde_json;

#[macro_use]
extern crate rand_derive;

extern crate byteorder;

extern crate spin_sleep;

#[macro_use]
extern crate derivative;

extern crate bincode;

extern crate itertools;

const API_VERSION: u32 = 0;


use self::enum_unitary::EnumUnitary;

use self::structopt::StructOpt;

use std::net::SocketAddr;

mod experiment;
mod numplay;
mod serve;
mod probe;


pub type Result<T> = ::std::result::Result<T, ::failure::Error>;

use crate::experiment::statement::ExperimentInfo;
use crate::experiment::statement::ExperimentReply;

#[derive(Debug, Serialize, Deserialize)]
struct ClientToServer {
    #[serde(flatten)]
    experiment: ExperimentInfo,
    api_version: u32,
}
#[derive(Debug, Serialize, Deserialize)]
struct ServerToClient {
    #[serde(flatten)]
    reply: ExperimentReply,
    api_version: u32,
}
impl From<ExperimentInfo> for ClientToServer {
    fn from(experiment: ExperimentInfo) -> Self { ClientToServer { experiment, api_version: API_VERSION }}
}
impl From<ExperimentReply> for ServerToClient {
    fn from(reply: ExperimentReply) -> Self { ServerToClient { reply, api_version: API_VERSION }}
}

#[derive(Debug, StructOpt)]
enum Cmd {
    /// Bind UDP socket for listening and start serving incoming experiments
    #[structopt(name = "serve")]
    Serve(serve::Cmd),

    /// Send experiment request to the specified UDP socket and do the experiment
    #[structopt(name = "probe")]
    Probe(probe::Cmd),

    /// Run some numeric experiment
    #[structopt(name = "n")]
    Numplay(numplay::Numplay),

    RDump,

    /// Output statistics saved by -R option of probe or serve
    #[structopt(name = "rawdump")]
    DumpSavedRawStats{
        #[structopt(parse(from_os_str))]
        file: ::std::path::PathBuf,
    },

    /// Summarize data from -R rawdump
    #[structopt(name = "analyse")]
    AnalyseRaw {
        #[structopt(parse(from_os_str))]
        file: ::std::path::PathBuf,
    }
}

fn main() -> Result<()> {
    let cmd = Cmd::from_args();

    match cmd {
        Cmd::Serve(x) => serve::serve(x)?,
        Cmd::Probe(x) =>  probe::probe(x)?,
        Cmd::Numplay(x) => numplay::numplay(x)?,
        Cmd::RDump => experiment::results::dump_some_results()?,
        Cmd::DumpSavedRawStats{file} => experiment::receiver::PacketReceiver::dump_raw_data(&file)?,
        Cmd::AnalyseRaw{file} => experiment::analyser::read_and_analyse(&file)?,
    };
    Ok(())
}
