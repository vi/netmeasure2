//! A battery of experiments

use ::structopt::StructOpt;
use crate::probe::{CmdImpl,CommunicOpts};
use crate::experiment::results::{ResultsForStoring,ExperimentResults};
use crate::experiment::statement::{ExperimentDirection,ExperimentInfo,ExperimentReply};
use ::rand::{XorShiftRng,RngCore,SeedableRng,Rng};
use crate::Result;


pub mod run;
pub mod generate;
pub mod visualise;

/// A battery of multiple probes
pub struct Battery(Vec<ExperimentInfo>);
