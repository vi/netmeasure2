use super::receiver::Info;
use super::results::{ExperimentResults,DelayModel,LossModel};
use crate::Result;
use super::results::{CLUSTERS,DELAY_DELTAS,DELAY_VALUES};


pub fn analyse(v: &[Info], total:usize) -> ExperimentResults {
    // Step 1: Sort, deduplicate byt seqn and get delays of input values
    let mut tmp : Vec<(u32,i32)> = Vec::with_capacity(v.len());
    let mut mindelay_ms = ::std::i32::MAX;
    for Info{seqn, st_us, rt_us} in v {
        let delay = (rt_us / 1000) as i32 - (st_us / 1000) as i32;
        tmp.push((*seqn, delay));
        if mindelay_ms > delay { mindelay_ms = delay }
    }
    for (_seqn,ref mut d) in tmp.iter_mut() {
        *d -= mindelay_ms;
    }
    tmp.sort();
    tmp.dedup_by_key(|(seqn,_d)|*seqn);

    // Step 2: Initialize results
    let mut r = ExperimentResults::default();
    // Assume no losses and no delay occur by default
    // (to prevent errors at normalisation step if no packets):
    const NONZERO_BUT_SMALL : f32 = 0.0001;
    r.loss_model.nonloss[CLUSTERS.len()-1]=NONZERO_BUT_SMALL;
    r.loss_model.loss[0]=NONZERO_BUT_SMALL;
    r.delay_model.value_popularity[0] = NONZERO_BUT_SMALL;
    r.delay_model.delta_popularity[0] = NONZERO_BUT_SMALL;

    //use ::rand::{FromEntropy,rngs::SmallRng};
    //let mut rnd = SmallRng::from_entropy();

    fn register(mut x: i32, v: &mut [f32], registry:&[i32] ) {
        //eprintln!("regcl {} in {:?}", x, v as *mut [f32]);
        match (registry.binary_search(&(x as i32))) {
            Ok(i) => v[i]+=1.0,
            Err(i) => {
                if i >= registry.len() {
                    v[registry.len() - 1] += 1.0;
                } else if i == 0 {
                    v[0] += 1.0;
                } else {
                    let prev = registry[i-1];
                    let next = registry[i];
                    let to_prev = x - prev;
                    let to_next = next - x;
                    assert!(to_prev > 0);
                    assert!(to_next > 0);
                    let to_next = to_next as f32;
                    let to_prev = to_prev as f32;
                    
                    let q = to_next / (to_prev + to_next);
                    
                    v[i-1] += q;
                    v[i] += 1.0-q;
                }
            },
        }
    }

    // Step 3: accumulate statistics about loss clusters
    let mut nonloss_in_a_row : u32 = 0;
    let mut prev_seqn = 0;
    for (seqn,_) in tmp.iter() {
        let jump_in_seqns = seqn - prev_seqn;
        if (jump_in_seqns <= 1) {
            nonloss_in_a_row+=1;
        } else {
            if nonloss_in_a_row > 0 {
                register(nonloss_in_a_row as i32, &mut r.loss_model.nonloss, &CLUSTERS);
            }
            nonloss_in_a_row = 0;
            let loss_cluster = jump_in_seqns - 1;
            register(loss_cluster as i32, &mut r.loss_model.loss, &CLUSTERS);
        }
        prev_seqn = *seqn;
    }
    if nonloss_in_a_row > 0 {
        register(nonloss_in_a_row as i32, &mut r.loss_model.nonloss,&CLUSTERS);
    }
    nonloss_in_a_row = 0;
    if prev_seqn + 1 < total as u32 {
        let last_loss_cluster = total as u32 - prev_seqn - 1;
        register(last_loss_cluster as i32, &mut r.loss_model.loss,&CLUSTERS);
    }

    // Step 4: accumulate statistics and delay values and deltas;
    let mut prevdelay = 0;
    for (_,d) in tmp.iter() {
        register(*d as i32, &mut r.delay_model.value_popularity,&DELAY_VALUES);
        register((*d - prevdelay) as i32, &mut r.delay_model.delta_popularity,&DELAY_DELTAS);
        prevdelay = *d;
    }

    // Step 5: Normalize results
    fn normalize(v:&mut [f32]) {
        let mut sum = 0.0;
        for &x in v.iter() {
            assert!(x>=0.0);
            sum += x;
        }
        assert!(sum>0.0);
        let q = 1.0 / sum;
        for x in v.iter_mut() {
            *x *= q;
        }
    }
    normalize(&mut r.loss_model.nonloss);
    normalize(&mut r.loss_model.loss);
    normalize(&mut r.delay_model.value_popularity);
    normalize(&mut r.delay_model.delta_popularity);
    r.total_received_packets=tmp.len() as u32;
    r.loss_model.loss_prob = 1.0 - tmp.len() as f32 / total as f32;
    r
}

pub fn read_and_analyse(p: &::std::path::Path) -> Result<()>  {
    let mut f = ::std::io::BufReader::new(::std::fs::File::open(p)?);
    let totpkt : usize = ::bincode::deserialize_from(&mut f)?;
    let v : Vec<Info> = ::bincode::deserialize_from(f)?;

    let r = analyse(&v, totpkt);

    println!(
        "Total received packets: {} (loss {:3.2}%)", 
        r.total_received_packets, 
        r.loss_model.loss_prob * 100.0,
    );
    r.visualise_loss();
    println!();
    r.visualise_delay();
    Ok(())
}