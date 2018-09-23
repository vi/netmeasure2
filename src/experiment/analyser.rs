use super::receiver::Info;
use super::results::{ExperimentResults,DelayModel,LossModel};
use crate::Result;
use super::results::{CLUSTERS,DELAY_DELTAS,DELAY_VALUES};


pub fn analyse(v: &[Info], total:usize) -> ExperimentResults {
    ExperimentResults::default()
}

pub fn read_and_analyse(p: &::std::path::Path) -> Result<()>  {
    let mut f = ::std::io::BufReader::new(::std::fs::File::open(p)?);
    let totpkt : usize = ::bincode::deserialize_from(&mut f)?;
    let v : Vec<Info> = ::bincode::deserialize_from(f)?;

    let r = analyse(&v, totpkt);

    println!(
        "Total received packets: {} (loss {:3.2}%)", 
        v.len(), 
        100.0 - v.len() as f64 * 100.0 / totpkt as f64,
    );
    r.visualise_loss();
    r.visualise_delay();
    Ok(())
}

fn bar(x:f32) -> &'static str {
    if x < 0.005 { return ""; }
    if x < 0.01 { return "-"; }
    if x < 0.02 { return "+"; }
    if x < 0.05 { return "#"; }
    if x < 0.10  { return "#-"; }
    if x < 0.15  { return "#+"; }
    if x < 0.20  { return "##"; }
    if x < 0.25  { return "##-"; }
    if x < 0.30  { return "##+"; }
    if x < 0.35  { return "###"; }
    if x < 0.40  { return "###-"; }
    if x < 0.45  { return "###+"; }
    if x < 0.50  { return "####"; }
    if x < 0.55  { return "####-"; }
    if x < 0.60  { return "####+"; }
    if x < 0.65  { return "#####"; }
    if x < 0.70  { return "#####-"; }
    if x < 0.75  { return "#####+"; }
    if x < 0.80  { return "######"; }
    if x < 0.850 { return "######-"; }
    if x < 0.900 { return "######+"; }
    if x < 0.950 { return "#######"; }
    if x < 0.950 { return "#######"; }
    if x < 0.990 { return "#######-"; }
    if x < 0.995 { return "#######+"; }
    "########"
}

impl ExperimentResults {
    pub fn visualise_loss(&self) {
        println!("      Loss:         |  Nonloss: ");
        let mut prevskipped = false;
        for (i,&c) in CLUSTERS.iter().enumerate() {
            let l = self.loss_model.loss[i];
            let nl = self.loss_model.nonloss[i];

            if l < 0.001 && nl < 0.001 {
                if !prevskipped {
                    println!("...");
                    prevskipped = true;
                }
                continue;
            } else {
                prevskipped = false;
            }

            if c != 65535 {
                print!("{:3}",c);
            } else {
                print!("UUU");
            }
            println!(
                " {:1.4} {:8} | {:1.4} {:8}",
                l,
                bar(l),
                nl,
                bar(nl),
            );
        }
    }

    pub fn visualise_delay(&self) {
        let mut delay_report = vec![];

        delay_report.push(format!("Delay:"));
        let mut prevskipped = false;
        for (i,&c) in DELAY_VALUES.iter().enumerate() {
            let v = self.delay_model.value_popularity[i];

            if v < 0.001 {
                if !prevskipped {
                    delay_report.push(format!("..."));
                    prevskipped = true;
                }
                continue;
            } else {
                prevskipped = false;
            }

            let header = if c != 65535 {
                format!("{:4}",c)
            } else {
                format!("UUUU")
            };
            delay_report.push(format!(
                "{} {:1.4} {:8}",
                header,
                v,
                bar(v),
            ));
        }

        let mut deltas_report = vec![];

        let mut deltas_values = vec![];

        for (i,&c) in DELAY_DELTAS.iter().enumerate() {
            let v = self.delay_model.delta_popularity[i];
            deltas_values.push((c,v));
        };
        deltas_values.sort_by_key(|(c,_v)|*c);

        deltas_report.push(format!("Delay deltas:"));
        let mut prevskipped = false;
        for (c,v) in deltas_values {
            if v < 0.001 {
                if !prevskipped {
                    deltas_report.push(format!("..."));
                    prevskipped = true;
                }
                continue;
            } else {
                prevskipped = false;
            }

            deltas_report.push(format!(
                "{:5} {:1.4} {:8}",
                c,
                v,
                bar(v),
            ));
        }
        use ::itertools::{Itertools,EitherOrBoth};
        for x in delay_report.iter().zip_longest(deltas_report.iter()) {
            match x {
                EitherOrBoth::Both(a,b) => {
                    println!(
                        "{:24} | {:24}",
                        a,
                        b,
                    );
                },
                EitherOrBoth::Left(a) => {
                    println!(
                        "{:24} | {:24}",
                        a,
                        "",
                    );
                },
                EitherOrBoth::Right(b) => {
                    println!(
                        "{:24} | {:24}",
                        "",
                        b,
                    );
                },
            }
        }
    }
}