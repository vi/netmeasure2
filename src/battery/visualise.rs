use ::structopt::StructOpt;
use crate::probe::{CmdImpl,CommunicOpts};
use crate::experiment::results::{ResultsForStoring,ExperimentResults};
use crate::experiment::statement::{ExperimentDirection,ExperimentInfo,ExperimentReply};
use ::rand::{XorShiftRng,RngCore,SeedableRng,Rng};
use crate::Result;
use super::Battery;


#[derive(Debug,EnumString)]
pub enum SortOrder {
    #[strum(serialize = "time")]
    Time,
    #[strum(serialize = "kbps")]
    Kbps,
    #[strum(serialize = "size")]
    PktSize,
    #[strum(serialize = "rate")]
    PktRate,
}

#[derive(StructOpt,Debug)]
pub struct BatteryShow {
    #[structopt(parse(from_os_str))]
    file: ::std::path::PathBuf,

    #[structopt(long="verbose",short="v")]
    verbose: bool,
}

impl BatteryShow {
    pub fn run(&self) -> Result<()> {
        print_summary(&self.file, self.verbose)
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

impl Battery {
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


pub fn print_summary(p: &::std::path::Path, verbose: bool) -> Result<()> {
    let f = ::std::io::BufReader::new(::std::fs::File::open(p)?);
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

pub fn migrate(p: &::std::path::Path) -> Result<()> {
    let f = ::std::io::BufReader::new(::std::fs::File::open(p)?);
    let v : Vec<ResultsForStoring> = ::serde_json::from_reader(f)?;
    ::serde_json::ser::to_writer(&mut ::std::io::stdout().lock(), &v)?;
    Ok(())
}
