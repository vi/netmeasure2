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
extern crate miniserde;

#[macro_use]
extern crate miniserde_tools;

#[macro_use]
extern crate counted_array;

#[macro_use]
extern crate static_assertions;

#[macro_use] 
extern crate serde_derive;

extern crate serde_cbor;

#[macro_use]
extern crate rand_derive;


use self::enum_unitary::EnumUnitary;

use self::structopt::StructOpt;

use std::net::SocketAddr;

mod experiment;
mod numplay;
mod serve;


pub type Result<T> = ::std::result::Result<T, ::failure::Error>;

#[derive(Debug, StructOpt)]
struct Probe {
    #[structopt(flatten)]
    experiment: crate::experiment::statement::ExperimentInfo,

    /// Remote UDP port to use as netmeasure2 server
    server: SocketAddr,
}

#[derive(Debug, StructOpt)]
enum Cmd {
    /// Bind UDP socket for listening and start serving incoming experiments
    #[structopt(name = "serve")]
    Serve(serve::Cmd),

    /// Send experiment request to the specified UDP socket and do the experiment
    #[structopt(name = "probe")]
    Probe(Probe),

    /// Run some numeric experiment
    #[structopt(name = "n")]
    Numplay(numplay::Numplay),

    RDump,
}

fn main() -> Result<()> {
    let cmd = Cmd::from_args();

    match cmd {
        Cmd::Serve(x) => serve::serve(x)?,
        Cmd::Probe(x) => println!("probe {}", miniserde::json::to_string(&x.experiment)),
        Cmd::Numplay(x) => numplay::numplay(x)?,
        Cmd::RDump => experiment::results::dump_some_results()?,
    };
    Ok(())
}
