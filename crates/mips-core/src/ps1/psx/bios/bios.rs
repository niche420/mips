use crate::error::*;
use crate::ps1::Ps1Error;
use crate::ps1::psx::bios::metadata;
use crate::ps1::psx::bios::metadata::{lookup_blob, Metadata};
use crate::ps1::util::ds::box_slice::BoxSlice;

pub struct Bios {
    rom: BoxSlice<u8, BIOS_SIZE>,
    metadata: &'static Metadata,
}

impl Bios {
    /// Create a BIOS image from `binary` and attempt to match it with an entry in the database. If
    /// no match can be found return an error.
    pub fn new(rom: BoxSlice<u8, BIOS_SIZE>) -> MipsResult<Bios> {
        match lookup_blob(&rom) {
            Some(metadata) => Ok(Bios {
                rom,
                metadata,
            }),
            None => Err(MipsError::from(Ps1Error::UnknownBios(String::from("Bios not supported"))))
        }
    }

    /// Return a static pointer to the BIOS's Metadata
    pub fn metadata(&self) -> &'static Metadata {
        self.metadata
    }
    
    /// Return the raw BIOS ROM
    pub fn rom(&self) -> &[u8; BIOS_SIZE] {
        &self.rom
    }

    /// Creates a BIOS instance with content set to all 0s.
    #[allow(dead_code)]
    pub fn new_dummy() -> Bios {
        let rom = BoxSlice::from_vec(vec![0; BIOS_SIZE]);

        Bios {
            rom,
            metadata: &metadata::DATABASE[0],
        }
    }

    /// Attempt to modify the BIOS ROM to replace the call to the code
    /// responsible for the boot logo animations by the provided
    /// instruction.
    pub fn patch_animation_jump_hook(&mut self,
                                     instruction: u32) -> MipsResult<()> {
        match self.metadata.animation_jump_hook {
            Some(h) => {
                let h = h as usize;

                self.rom[h]     = instruction as u8;
                self.rom[h + 1] = (instruction >> 8) as u8;
                self.rom[h + 2] = (instruction >> 16) as u8;
                self.rom[h + 3] = (instruction >> 24) as u8;

                Ok(())
            }
            None => Err(MipsError::from(Ps1Error::PatchBiosFailed))
        }
    }
}

/// 512 KB
pub const BIOS_SIZE: usize = 512 * 1024;
