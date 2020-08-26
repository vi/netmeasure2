use super::receiver::Info;
use super::results::{DelayModel, ExperimentResults, LossModel, ResultsForStoring};
use super::results::{CLUSTERS, DELAY_DELTAS, DELAY_VALUES};
use crate::Result;

fn bar(mut x: f32) -> &'static str {
    if x < 0.0 {
        x = -x
    };
    if x < 0.005 {
        return "";
    }
    if x < 0.01 {
        return "-";
    }
    if x < 0.02 {
        return "+";
    }
    if x < 0.05 {
        return "#";
    }
    if x < 0.10 {
        return "#-";
    }
    if x < 0.15 {
        return "#+";
    }
    if x < 0.20 {
        return "##";
    }
    if x < 0.25 {
        return "##-";
    }
    if x < 0.30 {
        return "##+";
    }
    if x < 0.35 {
        return "###";
    }
    if x < 0.40 {
        return "###-";
    }
    if x < 0.45 {
        return "###+";
    }
    if x < 0.50 {
        return "####";
    }
    if x < 0.55 {
        return "####-";
    }
    if x < 0.60 {
        return "####+";
    }
    if x < 0.65 {
        return "#####";
    }
    if x < 0.70 {
        return "#####-";
    }
    if x < 0.75 {
        return "#####+";
    }
    if x < 0.80 {
        return "######";
    }
    if x < 0.850 {
        return "######-";
    }
    if x < 0.900 {
        return "######+";
    }
    if x < 0.950 {
        return "#######";
    }
    if x < 0.950 {
        return "#######";
    }
    if x < 0.990 {
        return "#######-";
    }
    if x < 0.995 {
        return "#######+";
    }
    "########"
}

fn print_side_by_side1(a: &[String], b: &[String]) -> Vec<String> {
    use ::itertools::{EitherOrBoth, Itertools};
    let mut v = Vec::with_capacity(a.len() + b.len());
    for x in a.iter().zip_longest(b.iter()) {
        match x {
            EitherOrBoth::Both(a, b) => {
                v.push(format!("{:41} | {:41}", a, b,));
            }
            EitherOrBoth::Left(a) => {
                v.push(format!("{:41} | {:41}", a, "",));
            }
            EitherOrBoth::Right(b) => {
                v.push(format!("{:41} | {:41}", "", b,));
            }
        }
    }
    v
}
fn print_side_by_side(a: &[String], b: &[String]) {
    for l in &print_side_by_side1(a, b) {
        println!("{}", l);
    }
}

impl ExperimentResults {
    pub fn visualise_loss(&self) {
        let mut loss_report = vec![];
        let mut nonloss_report = vec![];

        println!(
            "Lost packets at the beginning: {}, at the end: {}",
            self.loss_model.begin_lp, self.loss_model.end_lp,
        );
        loss_report.push(format!("Loss:"));
        {
            let mut prevskipped = false;
            for (i, &c) in CLUSTERS.iter().enumerate() {
                let l = self.loss_model.loss[i];

                if l < 0.001 {
                    if !prevskipped {
                        loss_report.push(format!("..."));
                        prevskipped = true;
                    }
                    continue;
                } else {
                    prevskipped = false;
                }

                let header = if c != 65535 {
                    format!("{:3}", c)
                } else {
                    format!("UUU")
                };

                loss_report.push(format!("{:3} {:1.4} {:8}", header, l, bar(l),));
            }
        }
        nonloss_report.push(format!("Nonloss:"));
        {
            let mut prevskipped = false;
            for (i, &c) in CLUSTERS.iter().enumerate() {
                let nl = self.loss_model.nonloss[i];

                if nl < 0.001 {
                    if !prevskipped {
                        nonloss_report.push(format!("..."));
                        prevskipped = true;
                    }
                    continue;
                } else {
                    prevskipped = false;
                }

                let header = if c != 65535 {
                    format!("{:3}", c)
                } else {
                    format!("UUU")
                };

                nonloss_report.push(format!("{:3} {:1.4} {:8}", header, nl, bar(nl),));
            }
        }
        print_side_by_side(&loss_report, &nonloss_report);
    }

    pub fn visualise_delay(&self) {
        let mut delay_report = vec![];

        delay_report.push(format!(
            "Delay (mean={:.0}ms):",
            self.delay_model.mean_delay_ms
        ));
        let mut prevskipped = false;
        for (i, &c) in DELAY_VALUES.iter().enumerate() {
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
                format!("{:4}", c)
            } else {
                format!("UUUU")
            };
            delay_report.push(format!("{} {:1.4} {:8}", header, v, bar(v),));
        }

        let mut deltas_report1 = vec![];
        let mut deltas_report2 = vec![];

        let mut deltas_values = vec![];

        for (i, &c) in DELAY_DELTAS.iter().enumerate() {
            let d1 = self.delay_model.delta_noloss[i];
            let d2 = self.delay_model.delta_loss1[i];
            let d3 = self.delay_model.delta_loss2_20[i];
            let d4 = self.delay_model.delta_lossmany[i];
            deltas_values.push((c, (d1, d2, d3, d4)));
        }
        deltas_values.sort_by_key(|(c, _)| *c);

        let mut prevskipped;
        macro_rules! chunk_of_code {
            ($c:ident, $v:ident, $report:ident) => {
                let v = *$v;
                let c = *$c;
                let vv = v * (c as f32);
                if v < 0.001 && vv < 0.01 {
                    if !prevskipped {
                        $report.push(format!("..."));
                        prevskipped = true;
                    }
                    continue;
                } else {
                    prevskipped = false;
                }

                $report.push(format!(
                    "{:5} {:1.4} {:8}  @ {:+3.4} {:8}",
                    c,
                    v,
                    bar(v),
                    vv,
                    bar(vv / 10.0),
                ));
            };
        }

        deltas_report1.push(format!(
            "Delay deltas (no loss):",
            //self.latchiness(),
        ));
        prevskipped = false;
        for (ref c, (ref v, _, _, _)) in &deltas_values {
            chunk_of_code!(c, v, deltas_report1);
        }

        deltas_report2.push(format!(
            "Delay deltas (l 1 pkt):",
            //self.latchiness(),
        ));
        prevskipped = false;
        for (ref c, (_, ref v, _, _)) in &deltas_values {
            chunk_of_code!(c, v, deltas_report2);
        }

        deltas_report1.push(format!(
            "Delay deltas (l 2-20 pkt):",
            //self.latchiness(),
        ));
        prevskipped = false;
        for (ref c, (_, _, ref v, _)) in &deltas_values {
            chunk_of_code!(c, v, deltas_report1);
        }

        deltas_report2.push(format!(
            "Delay deltas (l many pkt):",
            //self.latchiness(),
        ));
        prevskipped = false;
        for (ref c, (_, _, _, ref v)) in &deltas_values {
            chunk_of_code!(c, v, deltas_report2);
        }

        let deltas_report = print_side_by_side1(&deltas_report1, &deltas_report2);
        print_side_by_side(&delay_report, &deltas_report);
    }
}

impl ResultsForStoring {
    pub fn print_to_stdout(&self) {
        println!("Experiment params: {:?}", self.conditions);
        println!("RTT={}ms", self.rtt_us / 1000);
        fn q(r: &ExperimentResults) {
            println!(
                "Total received packets: {} (loss {:3.2}%, send-side loss: {:3.2}%)",
                r.total_received_packets,
                r.loss_model.loss_prob * 100.0,
                r.loss_model.sendside_loss * 100.0,
            );
            r.visualise_loss();
            println!();
            r.visualise_delay();
            println!();
        }
        if let Some(ref to_server) = self.to_server {
            println!("** To server: ***");
            q(to_server);
        };
        if let Some(ref from_server) = self.from_server {
            println!("** From server: ***");
            q(from_server);
        };
        use crate::experiment::SmallishDuration;
        println!(
            "Data usage: {:.3} MiB, bitrate: {:.3} mbit/s",
            self.conditions.bytes_used() as f32 * 0.001 * 0.001,
            self.conditions.kbps() as f32 / 1000.0,
        );
    }
}

pub fn read_and_visualize(p: &::std::path::Path) -> Result<()> {
    let f = ::std::io::BufReader::new(::std::fs::File::open(p)?);
    let r: ResultsForStoring = ::serde_json::from_reader(f)?;
    r.print_to_stdout();
    Ok(())
}
