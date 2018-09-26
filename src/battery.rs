use ::structopt::StructOpt;
use crate::probe::{CmdImpl,CommunicOpts};
use crate::experiment::results::ResultsForStoring;
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

    pub fn run(self, cmd:Cmd) -> Result<()> {
        let mut v = vec![];

        let n = self.0.len();
        let co = cmd.co;

        eprintln!("0%");
        for (i,experiment) in self.0.into_iter().enumerate() {
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
                        retries += 1;
                        if retries == 3 {
                            bail!("Three fails in a row");
                        } else {
                            ::std::thread::sleep(::std::time::Duration::from_secs(20));
                            continue;
                        }
                    },
                }
            }

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
}

pub fn print_summary(p: &::std::path::Path) -> Result<()> {
    let mut f = ::std::io::BufReader::new(::std::fs::File::open(p)?);
    let v : Vec<ResultsForStoring> = ::serde_json::from_reader(f)?;

    use ::std::collections::BTreeMap;
    let mut m : BTreeMap<u32, usize> = BTreeMap::new();

    for (i,entry) in v.iter().enumerate() {
        let kbps = entry.conditions.kbps();
        m.insert(kbps, i);
    }

    println!("  kbps  | pktsz ||  loss_^ | delay_^ || loss_v | delay_v");
    for (_, &i) in m.iter() {
        let entry = &v[i];
        match entry.conditions.direction {
            ExperimentDirection::Bidirectional => { println!(
                "{:7} | {:5} || {:7.1} | {:7.0} || {:6.1} | {:7.0}",
                entry.conditions.kbps(),
                entry.conditions.packetsize,
                entry.to_server.as_ref().unwrap().loss_model.loss_prob*100.0,
                entry.to_server.as_ref().unwrap().delay_model.mean_delay_us / 1000.0,
                entry.from_server.as_ref().unwrap().loss_model.loss_prob*100.0,
                entry.from_server.as_ref().unwrap().delay_model.mean_delay_us / 1000.0,
            );},
            ExperimentDirection::ToServerOnly => { println!(
                "{:7} | {:5} || {:7.1} | {:7.0} || {:6.1} | {:7.0}",
                entry.conditions.kbps(),
                entry.conditions.packetsize,
                entry.to_server.as_ref().unwrap().loss_model.loss_prob*100.0,
                entry.to_server.as_ref().unwrap().delay_model.mean_delay_us / 1000.0,
                "",
                "",
            );},
            ExperimentDirection::FromServerOnly => { println!(
                "{:7} | {:5} || {:7.1} | {:7.0} || {:6.1} | {:7.0}",
                entry.conditions.kbps(),
                entry.conditions.packetsize,
                "",
                "",
                entry.from_server.as_ref().unwrap().loss_model.loss_prob*100.0,
                entry.from_server.as_ref().unwrap().delay_model.mean_delay_us / 1000.0,
            );},
        }
    }
    Ok(())
}