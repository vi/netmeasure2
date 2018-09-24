use ::std::net::{UdpSocket,SocketAddr};
use crate::Result;
use ::std::time::{Instant,Duration};
use ::byteorder::{BE,ByteOrder};
use crate::experiment::SmallishDuration;

fn now() -> Instant { Instant::now() }

pub struct Sender {
    pub packetsize: usize,
    pub rtpmimic: bool,
    pub packetcount: u32,
    pub experiment_start: Instant,
    pub delay_between_packets: Duration,
}

impl Sender {

    /// returns number of lost packets
    pub fn run(&self, udp: UdpSocket, to:SocketAddr) -> Result<u32> {
        assert!(self.packetsize >= 16);

        let mut sleeper = ::spin_sleep::SpinSleeper::default();
        
        let mut pkt = vec![0; self.packetsize];

        let mut lost = 0;

        eprintln!("Sender started");

        let mut hyst = false;

        for seqn in 0..self.packetcount {
            let n = now();
            let mut t = self.experiment_start + self.delay_between_packets * seqn;
            if n < t {
                sleeper.sleep(t - n);
                //::std::thread::sleep(t-n);
                //::spin_sleep::sleep(t-n);
                hyst = false;
            } else {
                if hyst || (n - t).as_us() > 1000_20 {
                    lost += 1;
                    hyst = true;
                    continue;
                }
            }

            let n = now();
            let mut ts = 0;
            if n > self.experiment_start { 
                ts = (n - self.experiment_start).as_us()
            };
            BE::write_u32(&mut pkt[8..12], seqn);
            BE::write_u32(&mut pkt[12..16], ts);
            
            udp.send_to(&pkt[..], to)?;
        }
        eprintln!("Sender stopped");

        Ok(lost)
    }

}