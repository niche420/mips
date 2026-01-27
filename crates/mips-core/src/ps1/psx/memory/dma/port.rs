use crate::ps1::psx::bus::Bus;
use crate::ps1::psx::graphics::gpu;
use crate::ps1::psx::{cd, mdec};
use crate::ps1::psx::memory::dma::Port;
use crate::ps1::psx::processor::ClockCycle;
use crate::ps1::psx::sound::spu;

/// Perform a DMA port write. Returns the overhead of the write
pub fn store(bus: &mut Bus, port: Port, v: u32) -> ClockCycle {
    match port {
        Port::Spu => {
            spu::dma_store(bus, v);
            // XXX Mednafen has a long comment explaining where this value comes from (and mention
            // that the average should be closer to 96). This is of course a wildly inaccurate
            // approximation but let's not worry about that for the time being.
            47
        }
        Port::Gpu => {
            gpu::dma_store(bus, v);
            0
        }
        Port::MDecIn => {
            mdec::dma_store(bus, v);
            0
        }
        _ => unimplemented!("DMA port store {:?}", port),
    }
}

/// Perform a DMA port read and returns the value alongside with the write offset (for MDEC, 0
/// elsewhere) and the delay penalty for the read
pub fn load(bus: &mut Bus, port: Port) -> (u32, u32, ClockCycle) {
    let mut offset = 0;
    let mut delay = 0;

    let v = match port {
        Port::Otc => {
            let channel = &bus.dma[port];

            if channel.remaining_words == 1 {
                // Last entry contains the end of table marker
                0xff_ffff
            } else {
                // Pointer to the previous entry
                channel.cur_address.wrapping_sub(4) & 0x1f_ffff
            }
        }
        // XXX latency taken from mednafen
        Port::CdRom => {
            delay = 8;
            cd::dma_load(bus)
        }
        Port::Spu => spu::dma_load(bus),
        Port::MDecOut => {
            let (v, off) = mdec::dma_load(bus);
            offset = off;
            v
        }
        Port::Gpu => gpu::dma_load(bus),
        _ => unimplemented!("DMA port load {:?}", port),
    };

    (v, offset, delay)
}

