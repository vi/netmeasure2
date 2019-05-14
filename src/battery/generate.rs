use ::structopt::StructOpt;
use crate::probe::{CmdImpl,CommunicOpts};
use crate::experiment::results::{ResultsForStoring,ExperimentResults};
use crate::experiment::statement::{ExperimentDirection,ExperimentInfo,ExperimentReply};
use ::rand::{RngCore,SeedableRng,Rng};
use ::rand::seq::SliceRandom;
use ::rand_xorshift::XorShiftRng;
use crate::Result;
use super::Battery;



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
            let packetsize = if r.gen_bool(0.5) {
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
            let packetsize = if r.gen_bool(0.5) {
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
        v[..].shuffle(&mut r);

        Battery(v)
    }
}
