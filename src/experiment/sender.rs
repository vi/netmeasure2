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

    pub fn run(&self, udp: UdpSocket, to:SocketAddr) -> Result<()> {
        assert!(self.packetsize >= 16);

        let mut sleeper = ::spin_sleep::SpinSleeper::default();
        
        let mut pkt = vec![0; self.packetsize];

        for seqn in 0..self.packetcount {
            let n = now();
            let mut t = self.experiment_start + self.delay_between_packets * seqn;
            if n < t {
                sleeper.sleep(t - n);
            }

            let n = now();
            BE::write_u32(&mut pkt[8..12], seqn);
            BE::write_u32(&mut pkt[12..16], (n - self.experiment_start).as_us());
            
            udp.send_to(&pkt[..], to)?;
        }

        Ok(())
    }

}