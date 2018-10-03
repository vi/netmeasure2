use ::structopt::StructOpt;
use crate::probe::{CmdImpl,CommunicOpts};
use crate::experiment::results::{ResultsForStoring,ExperimentResults};
use crate::experiment::statement::{ExperimentDirection,ExperimentInfo,ExperimentReply};
use ::rand::{XorShiftRng,RngCore,SeedableRng,Rng};
use crate::Result;

/// A battery of multiple probes
pub struct Battery(Vec<ExperimentInfo>);

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

fn getrand() -> XorShiftRng {
    let seed = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16];
    let r : XorShiftRng = SeedableRng::from_seed(seed);
    r
}


impl ::rand::distributions::Distribution<ExperimentDirection> for ::rand::distributions::Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ExperimentDirection {
        match rng.gen_range(0, 3) {
            0 => ExperimentDirection::Bidirectional,
            1 => ExperimentDirection::ToServerOnly,
            _ => ExperimentDirection::FromServerOnly,
        }
    }
}

impl Battery {
    pub fn generate() -> Self {
        let mut v = vec![];

        let mut r = getrand();

        let mut lightweight = 0;
        let mut mid1weight = 0;
        let mut mid2weight = 0;
        let mut heavyweight = 0;

        while v.len() < 50 {
            let mut packetsize = if r.gen_bool(0.5) {
                r.gen_range(100,1537)
            } else {
                r.gen_range(32,100)
            };
            let direction = r.gen();
            let packetdelay_us = if r.gen_bool(0.4) {
                r.gen_range(300, 2000)
            } else {
                r.gen_range(2000, 200_000)
            };
            let rtpmimic = r.gen();
            let mut totalpackets = (5_000_000 / packetdelay_us) as u32;
            if totalpackets < 1000 &&  r.gen_bool(0.7) { 
                totalpackets = 1000
            };
            if totalpackets < 200 {
                totalpackets = 200
            };
            let e = ExperimentInfo {
                direction,
                packetdelay_us,
                packetsize,
                pending_start_in_microseconds: 2000_000,
                rtpmimic,
                session_id: 0,
                totalpackets,
            };
            if r.gen_bool(0.8) && e.kbps() > 1000 {
                continue;
            }
            if e.kbps() > 1000_0 {
                continue;
            }
            if r.gen_bool(0.8) && e.duration().as_secs() > 10 {
                continue;
            }
            if e.duration().as_secs() > 30 {
                continue;
            }

            if e.kbps() < 20 {
                if lightweight < 15 {
                    lightweight += 1;
                } else {
                    continue;
                }
            } else if e.kbps() < 400 {
                if mid1weight < 15 {
                    mid1weight += 1;
                } else {
                    continue;
                }
            } else if e.kbps() < 1500 {
                if mid2weight < 15 {
                    mid2weight += 1;
                } else {
                    continue;
                }
            } else {
                if heavyweight < 10 {
                    heavyweight += 1;
                } else {
                    continue;
                }
            }

            v.push(e);
        }

        Battery(v)
    }


    pub fn generate_bb() -> Self {
        let mut v = vec![];

        let mut r = getrand();

        let mut lightweight = 0;
        let mut mid1weight = 0;
        let mut mid2weight = 0;
        let mut heavyweight = 0;

        while v.len() < 50 {
            let mut packetsize = if r.gen_bool(0.5) {
                r.gen_range(256,1537)
            } else {
                r.gen_range(80,256)
            };
            let direction = r.gen();
            let packetdelay_us = if r.gen_bool(0.5) {
                r.gen_range(40, 300)
            } else {
                r.gen_range(300, 30_000)
            };
            let rtpmimic = r.gen();
            let mut totalpackets = (5_000_000 / packetdelay_us) as u32;
            if totalpackets < 5000 &&  r.gen_bool(0.7) { 
                totalpackets = 5000
            };
            if totalpackets < 1000 {
                totalpackets = 1000
            };
            let e = ExperimentInfo {
                direction,
                packetdelay_us,
                packetsize,
                pending_start_in_microseconds: 2000_000,
                rtpmimic,
                session_id: 0,
                totalpackets,
            };
            if e.kbps() > 80_000 {
                continue;
            }
            if r.gen_bool(0.8) && e.duration().as_secs() > 10 {
                continue;
            }
            if e.duration().as_secs() > 30 {
                continue;
            }

            if e.kbps() < 200 {
                if lightweight < 15 {
                    lightweight += 1;
                } else {
                    continue;
                }
            } else if e.kbps() < 1400 {
                if mid1weight < 15 {
                    mid1weight += 1;
                } else {
                    continue;
                }
            } else if e.kbps() < 8000 {
                if mid2weight < 15 {
                    mid2weight += 1;
                } else {
                    continue;
                }
            } else {
                if heavyweight < 10 {
                    heavyweight += 1;
                } else {
                    continue;
                }
            }

            v.push(e);
        }
        r.shuffle(&mut v[..]);

        Battery(v)
    }

    pub fn show(&self) {
        let mut b = 0u64;
        let mut t = 0u64;
        for i in &self.0 {
            println!("{:.3}mbps {}s {:?}",  i.kbps() as f32/1000.0, i.duration().as_secs(), i);
            b += i.bytes_used() as u64;
            t += i.duration().as_secs() + 5;
            if i.direction == ExperimentDirection::Bidirectional {
                b += i.bytes_used() as u64; // once more
            }
        }
        println!("Total {} MiB, {} minutes, {} experiments", b / 1024/1024, t / 60, self.0.len());
    }
}

impl ResultsForStoring {
    pub fn short_summary(&self) -> String {
        let entry = self;

        let mut toserv = format!("");
        let mut fromserv = format!("");
        let q = |x : &ExperimentResults| {
            let lm = &x.loss_model;
            let ekbps = entry.conditions.kbps() as f32 * (1.0 - lm.loss_prob);
            let loss_sendside = if lm.sendside_loss * 2.0 <= lm.loss_prob {
                " "
            } else {
                "*"
            };
            let loss_recoverability = if lm.loss_prob < 0.01 {
                " "
            } else if (lm.loss[0] +
                       lm.loss[1] + 
                       lm.loss[2] + 
                       lm.loss[3] + 
                       lm.loss[4] + 
                       lm.loss[5] + 
                       lm.loss[6] + 
                       lm.loss[7] + 
                       lm.loss[8] + 
                       lm.loss[9] ) * lm.loss_prob >= 0.3 {
                "!"
            } else if lm.loss[0] >= 0.8 {
                "R"
            } else if lm.loss[0]+
                      lm.loss[1]+
                      lm.loss[2] >= 0.7 {
                "r"
            } else {
                " "
            };
            let lost_attheend = if lm.end_lp > 100 {
                "$"
            } else {
                " "
            };


            let latchup_marker = match (
                    (x.latchiness()*1000.0) as i32,
                    (x.delay_abrupt_decreaseness()*1000.0) as i32,
             ) {
                (-1000 ... 200, -1000...200) => "  ",
                (200 ... 2_000, -1000...200) => ". ",
                (2_000...5_000, -1000...200) => "l ",
                (5_000...10_000,-1000...200) => "L ",
                (10_000 ... 100000000, -100...2000) => "LL",

                (-1000 ... 200, 200...2000) => " ,",
                (200 ... 2_000, 200...2000) => ".,",
                (2_000...5_000, 200...2000) => "l,",
                (5_000...10_000,200...2000) => "L,",

                (-1000 ... 200, 2000...5000) => " r",
                (200 ... 2_000, 2000...5000) => ".r",
                (2_000...5_000, 2000...5000) => "lr",
                (5_000...10_000,2000...5000) => "Lr",

                (-1000 ... 200, 5000...10000) => " R",
                (200 ... 2_000, 5000...10000000) => ".R",
                (2_000...5_000, 5000...10000000) => "lR",
                (5_000...10_00000000,5000...10000000) => "LR",
                (-1000...2000,5000...10000000) => "RR",

                _ => "??",
            };
            format!(
                "{:7.0} | {:4.1}{}{}{}| {:7.0} {}",
                ekbps,
                lm.loss_prob*100.0,
                loss_sendside,
                loss_recoverability,
                lost_attheend,
                x.delay_model.mean_delay_ms,
                latchup_marker,
            )
        };
        if let Some(x) = entry.to_server.as_ref() {
            toserv = q(x);
        }
        if let Some(x) = entry.from_server.as_ref() {
            fromserv = q(x);
        }
        let rtpmim = if entry.conditions.rtpmimic {
            "R"
        } else {
            " "
        };
        format!(
            "{}{:6} | {:5} || {:29} || {:29}",
            rtpmim,
            entry.conditions.kbps(),
            entry.conditions.packetsize,
            toserv,
            fromserv,
        )
    }
}

pub fn print_summary(p: &::std::path::Path, verbose: bool) -> Result<()> {
    let mut f = ::std::io::BufReader::new(::std::fs::File::open(p)?);
    let v : Vec<ResultsForStoring> = ::serde_json::from_reader(f)?;

    use ::std::collections::BTreeMap;
    let mut m : BTreeMap<u32, usize> = BTreeMap::new();

    for (i,entry) in v.iter().enumerate() {
        let kbps = entry.conditions.kbps();
        m.insert(kbps, i);
    }

    println!("  kbps  | pktsz || ekbps_^ | loss_^ | delay_^    || ekbps_v | loss_v | delay_v  ");
    for (_, &i) in m.iter() {
        let entry = &v[i];
        println!("{}",entry.short_summary());
        if verbose {
            entry.print_to_stdout();
        }
    }
    Ok(())
}

    pub fn run(cmd:Cmd) -> Result<()> {
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
                        if i == 0 { bail!("First experiment failed") }
                        if i < 3 {
                            if format!("{}",e).contains("busy") {
                                bail!("Server is probably busy with another session");
                            }
                        }
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
                let mut f = ::std::fs::File::create(pb)?;
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
