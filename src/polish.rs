//! Polish code pages from the time before standards took hold, each made by putting the 18 Polish
//! letters on top of another code page.

use crate::tables::{CP437, ISO_8859_1, MAC_ROMAN};

/// In the order of the positions below: Ą Ć Ę Ł Ń Ó Ś Ź Ż ą ć ę ł ń ó ś ź ż
const POLISH: [u16; 18] = [
    0x0104, 0x0106, 0x0118, 0x0141, 0x0143, 0x00D3, 0x015A, 0x0179, 0x017B, 0x0105, 0x0107, 0x0119,
    0x0142, 0x0144, 0x00F3, 0x015B, 0x017A, 0x017C,
];

/// Positions below 128 stay as they are in ASCII, so the letter is lost there.
const fn overlay(base: [u16; 128], positions: [u8; 18]) -> [u16; 128] {
    let mut table = base;
    let mut index = 0;
    while index < 18 {
        let position = positions[index];
        if position >= 128 {
            table[position as usize - 128] = POLISH[index];
        }
        index += 1;
    }
    table
}

// From DOS.
#[rustfmt::skip]
mod dos {
    use super::{overlay, CP437};

    pub const MAZOVIA: [u16; 128] = overlay(CP437, [143, 149, 144, 156, 165, 163, 152, 160, 161, 134, 141, 145, 146, 164, 162, 158, 166, 167]);
    pub const DHN: [u16; 128] = overlay(CP437, [128, 129, 130, 131, 132, 133, 134, 136, 135, 137, 138, 139, 140, 141, 142, 143, 145, 144]);
    pub const CSK: [u16; 128] = overlay(CP437, [128, 129, 130, 131, 132, 133, 134, 136, 135, 160, 161, 162, 163, 164, 165, 166, 168, 167]);
    pub const CYFROMAT: [u16; 128] = overlay(CP437, [128, 129, 130, 131, 132, 133, 134, 136, 135, 144, 145, 146, 147, 148, 149, 150, 152, 151]);
    pub const IINTE_ISIS: [u16; 128] = overlay(CP437, [128, 129, 130, 131, 132, 133, 134, 135, 136, 144, 145, 146, 147, 148, 149, 150, 151, 152]);
    pub const IEA_SWIERK: [u16; 128] = overlay(CP437, [143, 128, 144, 156, 165, 153, 235, 157, 146, 160, 155, 130, 159, 164, 162, 135, 168, 145]);
    pub const LOGIC: [u16; 128] = overlay(CP437, [128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145]);
    pub const MICROVEX: [u16; 128] = overlay(CP437, [143, 128, 144, 156, 165, 147, 152, 157, 146, 160, 155, 130, 159, 164, 162, 135, 168, 145]);
    pub const VENTURA: [u16; 128] = overlay(CP437, [151, 153, 165, 166, 146, 143, 142, 144, 128, 150, 148, 164, 167, 145, 162, 132, 130, 135]);
    pub const FIDO_MAZOVIA: [u16; 128] = overlay(CP437, [143, 149, 144, 156, 165, 163, 152, 160, 161, 134, 135, 145, 146, 164, 162, 158, 166, 167]);
    pub const SMC: [u16; 128] = overlay(CP437, [198, 188, 201, 157, 185, 250, 156, 225, 237, 230, 99, 197, 229, 165, 243, 215, 142, 158]);
    pub const ELWRO_JUNIOR: [u16; 128] = overlay(CP437, [193, 195, 197, 204, 206, 207, 211, 218, 217, 225, 227, 229, 236, 238, 239, 243, 250, 249]);
    pub const ATARI_CALAMUS: [u16; 128] = overlay(CP437, [193, 194, 195, 196, 197, 198, 199, 200, 201, 209, 210, 211, 212, 213, 214, 215, 216, 217]);
    pub const COREL_DRAW: [u16; 128] = overlay(CP437, [197, 242, 201, 163, 209, 211, 255, 225, 237, 229, 236, 230, 198, 241, 243, 165, 170, 186]);
    pub const COREL_2: [u16; 128] = overlay(CP437, [194, 199, 202, 206, 209, 211, 212, 218, 219, 226, 231, 234, 238, 241, 243, 244, 250, 251]);
    pub const ATM: [u16; 128] = overlay(CP437, [196, 199, 203, 208, 209, 211, 214, 218, 220, 228, 231, 235, 240, 241, 243, 246, 250, 252]);
    pub const TEXPL: [u16; 128] = overlay(CP437, [129, 130, 134, 138, 139, 211, 145, 153, 155, 161, 162, 166, 170, 171, 243, 177, 185, 187]);
}

// From Amiga and Mac.
#[rustfmt::skip]
mod others {
    use super::{overlay, ISO_8859_1, MAC_ROMAN};

    pub const AMIGA_PL: [u16; 128] = overlay(ISO_8859_1, [194, 202, 203, 206, 207, 211, 212, 218, 219, 226, 234, 235, 238, 239, 243, 244, 250, 251]);
    pub const AMIGA_FAT: [u16; 128] = overlay(ISO_8859_1, [192, 193, 194, 195, 196, 197, 198, 200, 199, 230, 231, 234, 238, 241, 243, 245, 251, 254]);
    pub const AMIGA_XJP: [u16; 128] = overlay(ISO_8859_1, [198, 199, 202, 206, 209, 211, 213, 219, 222, 230, 231, 234, 238, 241, 243, 245, 251, 254]);
    pub const MAC_PL: [u16; 128] = overlay(MAC_ROMAN, [132, 140, 162, 252, 193, 238, 229, 143, 251, 136, 141, 171, 184, 196, 151, 230, 144, 253]);
}

pub use dos::*;
pub use others::*;
