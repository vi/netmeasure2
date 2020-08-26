//! A battery of experiments

use crate::experiment::results::{ExperimentResults, ResultsForStoring};
use crate::experiment::statement::{ExperimentDirection, ExperimentInfo, ExperimentReply};
use crate::Result;
use ::structopt::StructOpt;

pub mod generate;
pub mod run;
pub mod visualise;

/// A battery of multiple probes
pub struct Battery(Vec<ExperimentInfo>);
