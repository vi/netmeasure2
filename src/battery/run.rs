use ::structopt::StructOpt;
use crate::probe::{CmdImpl,CommunicOpts};
use crate::experiment::results::{ResultsForStoring,ExperimentResults};
use crate::experiment::statement::{ExperimentDirection,ExperimentInfo,ExperimentReply};
use ::rand::{XorShiftRng,RngCore,SeedableRng,Rng};
use crate::Result;
use super::Battery;



#[derive(Debug,StructOpt)]
pub struct Cmd {
    #[structopt(flatten)]
    co : CommunicOpts,

    #[structopt(long="output", short="o", parse(from_os_str))]
    pub output: Option<::std::path::PathBuf>,

    /// Format results nicely to stdout
    /// (maybe in addition to outputing JSON to `-o` file)
    #[structopt(short="S")]
    visualise: bool,

    /// Do a big, half-a-gigabyte test for more broadband networks
    #[structopt(long="big")]
    big: bool,
    
    /// Do a normal test. This flag is no-op.
    #[structopt(long="small")]
    small: bool,

    /// Maximum number of retries if non-first experiment is failed
    #[structopt(long="max-retries", default_value="4")]
    max_retries: usize,
    
    /// Wait this number of seconds after single experiment failure before retrying
    #[structopt(long="wait-before-retry", default_value="30")]
    wait_before_retry: u64,
}


impl Cmd {
    pub fn run(self) -> Result<()> {
        let cmd = self;
        let mut v = vec![];

        let battery = if cmd.big {
            Battery::generate_bb()
        } else {
            Battery::generate()
        };

        let n = battery.0.len();
        let co = cmd.co;


        eprintln!("0%");
        for (i,experiment) in battery.0.into_iter().enumerate() {
            use crate::probe::probe_impl;

            let mut retries = 0;

            let ci = CmdImpl {
                co: co.clone(),
                experiment: experiment,
            };

            loop {
                match probe_impl(ci.clone()) {
                    Ok(r) => {
                        v.push(r);
                        break;
                    },
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        if i < 3 {
                            if format!("{}",e).contains("busy") {
                                bail!("Server is probably busy with another session");
                            }
                        }
                        if i == 0 { bail!("First experiment failed") }
                        retries += 1;
                        if retries == cmd.max_retries {
                            bail!("Too many fails in a row, exiting");
                        } else {
                            ::std::thread::sleep(::std::time::Duration::from_secs(
                                cmd.wait_before_retry,
                            ));
                            continue;
                        }
                    },
                }
            }

            eprintln!("{}",v[i].short_summary());
            eprintln!("{}%", (i+1) * 100 / n);
        }

        if cmd.visualise && cmd.output.is_none() {
            println!("Visualise not implemented");
        } else {
            let out : Box<dyn(::std::io::Write)>;
            if let Some(pb) = cmd.output {
                let f = ::std::fs::File::create(pb)?;
                out = Box::new(f);
                if cmd.visualise {
                    println!("Visualise not implemented");
                }
            } else {
                out = Box::new(::std::io::stdout());
            }
            let mut out = ::std::io::BufWriter::new(out);
            ::serde_json::ser::to_writer(&mut out, &v)?;
            use ::std::io::Write;
            writeln!(out);
        }

        Ok(())
    }
}