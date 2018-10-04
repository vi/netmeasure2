use ::std::net::{UdpSocket,SocketAddr};
use crate::Result;
use ::std::time::{Instant,Duration};
use ::byteorder::{BE,ByteOrder};
use crate::experiment::SmallishDuration;
use super::statement::MINPACKETSIZE;

fn now() -> Instant { Instant::now() }

pub struct Sender {
    pub packetsize: usize,
    pub rtpmimic: bool,
    pub packetcount: u32,
    pub experiment_start: Instant,
    pub delay_between_packets: Duration,
    pub session_id: u64,
}

impl Sender {

    /// returns number of lost packets
    pub fn run(&self, udp: UdpSocket, to:SocketAddr) -> Result<u32> {
        assert!(self.packetsize >= MINPACKETSIZE);

        let mut sleeper = ::spin_sleep::SpinSleeper::default();
        
        let mut pkt = vec![0; self.packetsize];

        let mut lost = 0;

        if self.rtpmimic {
            pkt[0] |= 2 << 6; // RTP version 2;
            pkt[0] |= 0 << 5; // no padding;
            pkt[0] |= 0 << 4; // no extensions;
            pkt[0] |= 0 << 0; // CSRC count;
            pkt[1] |= 0 << 7; // marker bit
            pkt[1] |= 100; // payload type
            // 2-byte sequence number pkt[2..4]
            // 4-byte timestamp pkt[4..8]
            BE::write_u32(&mut pkt[8..12], (self.session_id & 0xFFFF_FFFF) as u32); // ssrc
        }

        eprintln!("Sender started");

        for seqn in 0..self.packetcount {
            let n = now();
            let mut t = self.experiment_start + self.delay_between_packets * seqn;
            if n <= t {
                sleeper.sleep(t - n);
                //::std::thread::sleep(t-n);
                //::spin_sleep::sleep(t-n);
            } else {
                if (n - t).as_us() > 10_000 {
                    lost += 1;
                    continue;
                }
            }

            let n = now();
            let mut ts = 0;
            if n > self.experiment_start { 
                ts = (n - self.experiment_start).as_us()
            };
            BE::write_u32(&mut pkt[12..16], seqn);
            BE::write_u32(&mut pkt[16..20], ts);

            if self.rtpmimic {
                BE::write_u16(&mut pkt[2..4], (seqn & 0xFFFF) as u16);
                BE::write_u32(&mut pkt[4..8], ts * 90 / 1000);
            }
            
            if let Err(_) = udp.send_to(&pkt[..], to) {
                lost += 1;
            }
        }
        eprintln!("Sender stopped");

        Ok(lost)
    }

}