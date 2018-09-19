use ::structopt::StructOpt;

use ::rand::{XorShiftRng,RngCore,SeedableRng,Rng};
use ::rand::distributions::{Uniform};

use ::std::io::{Write,Read,BufRead};


#[derive(StructOpt,Debug)]
pub enum Numplay {
    Gen,
    Gen2,
    Check,
}


pub fn numplay(n : Numplay) -> ::std::io::Result<()> {
    match n {
        Numplay::Gen => gen(),
        Numplay::Gen2 => gen2(),
        Numplay::Check => check(),
    }
}

const TD : &[(u32,u32)] = &[
    (1, 5),
    (2, 20),
    (3, 20),
    (4, 5),
    (5, 20),
    (6, 20),
    (7, 5),
    (8, 5),
];
fn prsum() -> u32 { TD.iter().map(|(_,x)|x).sum() }

fn getrand() -> XorShiftRng {
    let seed = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16];
    let r : XorShiftRng = SeedableRng::from_seed(seed);
    r
}

fn gen() -> ::std::io::Result<()> {
    let mut r = getrand();

    let prsum = prsum();

    let initial = r.choose(TD).unwrap().0;

    let so = ::std::io::stdout();
    let so = so.lock();
    let mut so = ::std::io::BufWriter::new(so);

    let mut a = initial;
    loop {
        let mut c = r.sample(Uniform::new(0, prsum));

        for (v,w) in TD {
            if c < *w {
                writeln!(so, "{}", v)?;
                
                break;
            } else {
                c -= *w;
            }
        }
    }
}

fn check() -> ::std::io::Result<()> {
    let si = ::std::io::stdin();
    let si = si.lock();
    let mut si = ::std::io::BufReader::new(si);

    let mut stat : Vec<u32> = vec![];

    let mut ctr = 0usize;

    for l in si.lines() {
        let v : usize = l?.parse().unwrap_or(0);
        assert!(v!=::std::usize::MAX);
        if stat.len() < (v+1) {
            stat.resize_default((v+1));
        }
        stat[v] += 1;
        ctr += 1;
    }

    for (n,v) in stat.iter().enumerate() {
        println!("{: >3} {: >8.5}", n, (*v as f64) * 100.0 / (ctr as f64));
    }

    Ok(())
}

const MS : usize = 9;

fn gen2() -> ::std::io::Result<()> {
    let mut r = getrand();
    let prsum = prsum();

    let mut markov = vec![ vec![0.00001; MS]; MS];

    /*for (ov, _) in TD {
        for (nv, w) in TD {
            let (ov,nv) = (*ov,*nv);
            markov[ov as usize][nv as usize] += (*w as f64);
            markov[ov as usize][0] += markov[ov as usize][nv as usize];
        }
    };*/

    let mut markov2;
    markov2 = markov.clone();

    let initial = r.choose(TD).unwrap().0;

    let so = ::std::io::stdout();
    let so = so.lock();
    let mut so = ::std::io::BufWriter::new(so);

    let mut stat : Vec<u32> = vec![0u32; MS];
    let mut ctr = 0u32;

    let mut a : u32 = initial;
    loop {
        if ctr % 1 == 0 {
            let mut pressure = vec![0.0; MS];
            for i in 1..MS {
                pressure[i] = ((ctr+1) as f64 * TD[i-1].1 as f64 / prsum as f64) 
                    -
                    (stat[i] as f64);
            }
            //markov2 = markov.clone();
            for i in 1..MS {
                markov2[i][0] = 0.0;
                for j in 1..MS {
                    let mut q =   0.00001;
                    if j == i { q=0.00012; }
                    if j == i + 1 { q=1.0; }
                    if j == i - 1 { q=1.0; }
                    //if j == i - 1 { q=1.0; }
                    //if j == 8 && i == 1 { q=0.1 }
                    //if j == 1 && i == 8 { q=0.1 }
                    
                    let aa : f64 = 0.1; //r.sample(Uniform::new(0.01,0.2));
                    let mut y = markov2[i][j] * (1.0-aa) + aa*q*pressure[j];

                    if y < 0.00001 { y = 0.00001; }

                    markov2[i][j] = y;
                    markov2[i][0] += y;
                }
            }
        }

        let mut c = r.sample(Uniform::new(0.0, markov2[a as usize][0]));

        for (v,w) in markov2[a as usize][1..].iter().enumerate() {
            let v = v+1;
            if c < *w {
                writeln!(so, "{}", v)?;
                a = v as u32;
                stat[v]+=1;
                ctr+=1;
                break;
            } else {
                c -= *w;
            }
        }
    }
}