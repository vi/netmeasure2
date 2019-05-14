use ::structopt::StructOpt;
use crate::probe::{CmdImpl,CommunicOpts};
use crate::experiment::results::{ResultsForStoring,ExperimentResults};
use crate::experiment::statement::{ExperimentDirection,ExperimentInfo,ExperimentReply};
use ::rand::{RngCore,SeedableRng,Rng};
use ::rand_xorshift::XorShiftRng;
use crate::Result;
use super::Battery;


#[derive(Debug,EnumString,Clone,Copy)]
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

    /// kbps(default), rate (packet rate), size (packet size), time
    #[structopt(long="sort",short="s",default_value="kbps")]
    sort: SortOrder,
}

impl BatteryShow {
    pub fn run(&self) -> Result<()> {
        print_summary(&self.file, self.verbose, self.sort)
    }
}

// Summary of one half-experiment
pub struct Excerpt {
    pub kbps: f32,
    pub ekbps: f32,
    pub loss: f32,
    pub loss_sendside: bool,
    pub loss_sendsize_precise : f32,
    pub loss_recoverability: char,
    pub loss_at_the_end: bool,
    pub delay: f32,
    pub latchup_marker: &'static str,
}

impl Excerpt {
    pub fn format(&self) -> String {
        let loss = if self.loss > 0.999 {
            "100!".to_string()
        } else {
            format!("{:4.1}",self.loss*100.0)
        };
        format!(
            "{:7.0} | {}{}{}{}| {:7.0} {}",
            self.ekbps,
            loss,
            if self.loss_sendside { '*' } else { ' ' },
            self.loss_recoverability,
            if self.loss_at_the_end { '$' } else {' '},
            self.delay,
            self.latchup_marker,
        )
    }

    /// From 0.0 to 10.0
    pub fn quality_score(&self) -> f32 {
        let mut loss_karma = 0.0;
        if self.loss > 0.8 {
            return 0.0
        }
        loss_karma += self.loss - self.loss_sendsize_precise;
        loss_karma += 0.5 * self.loss_sendsize_precise;
        match self.loss_recoverability  {
            'R' => loss_karma *= 0.25,
            'r' => loss_karma *= 0.5,
            '!' => loss_karma *= 1.3,
            _ => (),
        }
        loss_karma *= 10.0;
        if loss_karma > 9.0 {
            return 0.0;
        };
        let mut delay_karma = match self.delay {
            x if x > 5000.0 => 10.0,
            x if x > 3000.0 => 6.0 + (10.0 - 6.0) * (x - 3000.0) / (5000.0-3000.0),
            x if x > 1000.0 => 4.0 + (6.0 - 4.0) * (x - 1000.0) / (3000.0-1000.0),
            x if x > 500.0 => 2.0 + (4.0 - 2.0) * (x - 500.0) / (1000.0-500.0),
            x => x / 500.0 * 2.0,
        };
        match self.latchup_marker {
            ". " => delay_karma += 0.4,
            ".," => delay_karma += 0.2,
            "l " => delay_karma += 1.0,
            "l," => delay_karma += 0.8,
            "lr" => delay_karma += 0.6,
            "lR" => delay_karma += 0.5,
            "L " => delay_karma += 2.0,
            "L," => delay_karma += 1.8,
            "Lr" => delay_karma += 1.5,
            "LR" => delay_karma += 1.2,
            "LL" => delay_karma += 2.5,
            _ => (),
        };
        if delay_karma > 10.0 {
            delay_karma = 10.0;
        };
        //println!("loss_karma={} delay_karma={}", loss_karma, delay_karma);
        let worst = delay_karma.max(loss_karma);
        let avg = (delay_karma + loss_karma*2.0) / 3.0;
        let mut adj = (avg + worst) / 2.0;
        if adj > 4.0 {
            adj = 4.0  + (adj - 4.0) / 6.0;
        }
        10.0 - 2.0 * adj
    }
}

impl ExperimentResults {
    /// Return short summarized results
    pub fn get_exceprt(&self, conditions: &ExperimentInfo) -> Excerpt {
        let x = self;
        let lm = &x.loss_model;
        let loss = lm.loss_prob;
        let kbps = conditions.kbps() as f32;
        let ekbps = conditions.kbps() as f32 * (1.0 - loss);
        let loss_sendside = if lm.sendside_loss * 2.0 <= lm.loss_prob {
            false
        } else {
            true
        };
        let loss_recoverability = if lm.loss_prob < 0.01 {
            ' '
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
            '!'
        } else if lm.loss[0] >= 0.8 {
            'R'
        } else if lm.loss[0]+
                    lm.loss[1]+
                    lm.loss[2] >= 0.7 {
            'r'
        } else {
            ' '
        };
        let loss_at_the_end = if lm.end_lp > 100 {
            true
        } else {
            false
        };

        let delay = x.delay_model.mean_delay_ms;
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
        Excerpt {
            kbps,
            ekbps,
            loss,
            loss_sendside,
            loss_recoverability,
            loss_at_the_end,
            delay,
            latchup_marker,
            loss_sendsize_precise: self.loss_model.sendside_loss,
        }
    }
}

impl ResultsForStoring {
    pub fn short_summary(&self) -> String {
        let entry = self;

        let mut toserv = format!("");
        let mut fromserv = format!("");
        let mut score = 10.0f32;
        let mut q = |x : &ExperimentResults| {
            let e = x.get_exceprt(&entry.conditions);
            score = score.min(e.quality_score());
            e.format()
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
            "{}{:6} | {:5} || {:29} || {:29}|| {:2.0}",
            rtpmim,
            entry.conditions.kbps(),
            entry.conditions.packetsize,
            toserv,
            fromserv,
            score,
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


pub fn print_summary(p: &::std::path::Path, verbose: bool, sort_order: SortOrder) -> Result<()> {
    let f = ::std::io::BufReader::new(::std::fs::File::open(p)?);
    let v : Vec<ResultsForStoring> = ::serde_json::from_reader(f)?;

    use ::std::collections::BTreeMap;
    let mut m : BTreeMap<u64, usize> = BTreeMap::new();

    for (i,entry) in v.iter().enumerate() {
        let sortkey = match sort_order {
            SortOrder::Kbps => entry.conditions.kbps() as u64,
            SortOrder::Time => i as u64,
            SortOrder::PktRate => entry.conditions.packetdelay_us as u64,
            SortOrder::PktSize => entry.conditions.packetsize as u64,
        };
        m.insert(sortkey, i);
    }

    println!("  kbps  | pktsz || ekbps_^ | loss_^ | delay_^    || ekbps_v | loss_v | delay_v   || score");
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
