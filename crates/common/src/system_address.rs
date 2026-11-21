//! What a system address says about where the system is.
//!
//! The game derives every system's address from its position, for hand-named systems like
//! Sol as much as for generated ones. The galaxy is cut into sectors of 1,280 Ly, and each
//! sector into cubic cells whose edge depends on the system's mass code: 10 Ly for code `a`
//! up to the whole sector for `h`. The address packs, from the lowest bit up:
//!
//! | field            | bits        |
//! |------------------|-------------|
//! | mass code `mc`   | 3           |
//! | z cell, z sector | 7 − mc, 7   |
//! | y cell, y sector | 7 − mc, 6   |
//! | x cell, x sector | 7 − mc, 7   |
//! | number in cell   | 11 + 3·mc   |
//! | body             | 9 (zero for a system) |
//!
//! So an address names one cube of space, and a position reported with it has to lie
//! inside. Measured against our own database on 2026-11-20: 5,334,220 of 5,334,225 known
//! systems do, and the other five were test messages with invented addresses.

/// Edge of a sector, in Ly.
const SECTOR_LY: u64 = 1280;
/// Edge of the smallest cell (mass code `a`), in Ly.
const SMALLEST_CELL_LY: u64 = 10;
/// The lowest corner of sector (0, 0, 0), in galactic coordinates (x, y, z).
const ORIGIN: [f64; 3] = [-49985.0, -40985.0, -24105.0];
/// Bits of the sector index along x, y and z: the galaxy is flatter than it is wide.
const SECTOR_BITS: [u32; 3] = [7, 6, 7];
/// The lowest bit of the body number; everything from here up is zero for a system.
const BODY_SHIFT: u32 = 55;

/// The cube of space a system address names.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    /// The corner with the lowest x, y and z, in Ly.
    pub corner: [f64; 3],
    /// Edge length in Ly: 10 for mass code `a`, doubling up to 1,280 for `h`.
    pub size: f64,
    /// Mass code, 0 (`a`) to 7 (`h`).
    pub mass_code: u8,
}

impl Cell {
    /// The cell of a system address, or `None` for a number that cannot be one: not
    /// positive, or with a body number in its top bits.
    pub fn of(address: i64) -> Option<Self> {
        if address <= 0 || address >> BODY_SHIFT != 0 {
            return None;
        }
        let a = address as u64;
        let mass_code = (a & 7) as u32;
        let cell_bits = 7 - mass_code;
        let cell_ly = SMALLEST_CELL_LY << mass_code;

        // The axes are packed z, y, x from the low end, each as cell index then sector index.
        let mut corner = [0.0; 3];
        let mut shift = 3;
        for axis in [2, 1, 0] {
            let cell = (a >> shift) & ((1 << cell_bits) - 1);
            shift += cell_bits;
            let sector = (a >> shift) & ((1 << SECTOR_BITS[axis]) - 1);
            shift += SECTOR_BITS[axis];
            corner[axis] = (sector * SECTOR_LY + cell * cell_ly) as f64 + ORIGIN[axis];
        }
        Some(Self { corner, size: cell_ly as f64, mass_code: mass_code as u8 })
    }

    /// Whether a position (x, y, z in Ly) lies in the cell, its faces included.
    pub fn contains(&self, pos: &[f32]) -> bool {
        pos.len() == 3
            && pos.iter().zip(self.corner).all(|(&p, low)| {
                let p = f64::from(p);
                p >= low && p <= low + self.size
            })
    }
}

/// Whether `pos` can be the position of the system with this address.
pub fn matches_position(address: i64, pos: &[f32]) -> bool {
    Cell::of(address).is_some_and(|cell| cell.contains(pos))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real systems from the edcas database, one per mass code and a few everyone knows.
    const REAL: &[(i64, &str, [f32; 3])] = &[
        (9153161034116104, "Lyaisua DL-Y a520", [-1333.8125, -6.03125, 30945.21875]),
        (3198033051342337, "Lyaisua ZE-A b1454", [-1305.625, 14.28125, 30935.53125]),
        (2766727636985434, "Byoomi SY-L c10-10065", [-1418.78125, -158.25, 24998.375]),
        (557703038472995, "Byoomi ZP-V d3-16231", [-1739.40625, -150.40625, 24860.21875]),
        (62896824355292, "Juenae HM-V e2-14644", [454.625, -540.78125, 26324.03125]),
        (3322860266549, "Dryaa Flyi GW-W f1-6189", [685.6875, -528.90625, 19006.71875]),
        (193089893022, "Scaulou FG-Y g2877", [-6543.75, -984.34375, 29270.90625]),
        (7359462647, "Floawns AA-A h877", [1597.90625, -1222.0625, 15171.09375]),
        (10477373803, "Sol", [0.0, 0.0, 0.0]),
        (164098653, "Achenar", [67.5, -119.46875, 24.84375]),
        (20578934, "Sagittarius A*", [25.21875, -20.90625, 25899.96875]),
        (3238296097059, "Colonia", [-9530.5, -910.28125, 19808.125]),
        (81973396946, "Beagle Point", [-1111.5625, -134.21875, 65269.75]),
        (1992026034234, "Plooe Thoi RB-O c6-7", [-9506.9375, 87.375, -3322.9375]),
    ];

    #[test]
    fn real_systems_lie_in_the_cell_of_their_address() {
        for &(address, name, pos) in REAL {
            let cell = Cell::of(address).unwrap_or_else(|| panic!("{name} has no cell"));
            assert!(cell.contains(&pos), "{name} at {pos:?} is outside {cell:?}");
        }
    }

    #[test]
    fn the_mass_code_is_the_letter_in_a_generated_name() {
        for (i, &(address, name, _)) in REAL.iter().take(8).enumerate() {
            let cell = Cell::of(address).unwrap();
            assert_eq!(cell.mass_code as usize, i, "{name}");
            assert_eq!(cell.size, (10 << i) as f64, "{name}");
        }
    }

    #[test]
    fn sols_cell_is_the_80_ly_cube_around_it() {
        let cell = Cell::of(10477373803).unwrap();
        assert_eq!(cell.corner, [-65.0, -25.0, -25.0]);
        assert_eq!(cell.size, 80.0);
    }

    /// What reached the edcas database over EDDN in 2026: invented addresses with
    /// placeholder positions, and real scans replayed under address 1.
    #[test]
    fn invented_addresses_do_not_match() {
        let fakes: &[(i64, [f32; 3])] = &[
            (1, [0.0, 0.0, 0.0]),                       // "BioAtlas test fixture"
            (2, [0.0, 0.0, 0.0]),                       // "Somewhere Else"
            (1000, [0.0, 0.0, 0.0]),                    // a second "Sol"
            (12345, [0.0, 0.0, 0.0]),                   // "sol"
            (5554443332221, [0.0, 0.0, 0.0]),           // "Target System"
            (123456789, [1.0, 2.0, 3.0]),               // "EDDash Test"
            (12345678901234567, [1.0, 2.0, 3.0]),       // "Test system"
            (1234567890, [-2482.96875, -335.8125, 1993.625]),
            (7268562208123, [6000.5, 150.25, 11000.75]),
            (9876543210987, [15.2, -8.4, 32.1]),
            (1, [-36.4375, 54.21875, 31.375]),          // LP 329-20's scans, sent as address 1
        ];
        for &(address, pos) in fakes {
            assert!(!matches_position(address, &pos), "{address} at {pos:?} must not match");
        }
    }

    #[test]
    fn a_real_address_does_not_match_another_systems_position() {
        // Sol's address with Achenar's position: both real, 139 Ly apart.
        assert!(!matches_position(10477373803, &[67.5, -119.46875, 24.84375]));
        // Just outside one face of Sol's cell, and exactly on it.
        assert!(!matches_position(10477373803, &[15.03125, 0.0, 0.0]));
        assert!(matches_position(10477373803, &[15.0, 0.0, 0.0]));
    }

    #[test]
    fn numbers_that_cannot_be_a_system_address_have_no_cell() {
        assert_eq!(Cell::of(0), None);
        assert_eq!(Cell::of(-10477373803), None);
        // Sol's address with body 3 in the top bits: a body's number, not a system's.
        assert_eq!(Cell::of(10477373803 | (3 << 55)), None);
        assert!(!matches_position(10477373803, &[0.0, 0.0]));
        assert!(!matches_position(10477373803, &[f32::NAN, 0.0, 0.0]));
    }
}
