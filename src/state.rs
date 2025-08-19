use num_enum::{IntoPrimitive, TryFromPrimitive};

const CRC: crc::Crc<u8> = crc::Crc::<u8>::new(&crc::CRC_8_OPENSAFETY);

/// Image slot ID.
///
/// Valid values from 0x00 to 0x06.
pub struct Slot(u8);

impl Slot {
    pub fn try_from_u8(val: u8) -> Option<Slot> {
        if val >= 0b111 {
            None
        } else {
            Some(Slot(val))
        }
    }
}

impl From<Slot> for u8 {
    fn from(val: Slot) -> Self {
        val.0
    }
}

#[derive(Debug)]
pub enum ParseResult {
    /// Nor flash entry yet to be written.
    Unset,
    /// State is an invalid value.
    Invalid,
}

/// Boot process status as stored in [State] as a 2-bit field.
///
/// The enum values are assigned such that bits can be dropped for the happy flow,
/// ensuring minimal wear on the storage.
#[derive(Debug, PartialEq, Clone, Copy, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum Status {
    /// Initial attempt at booting the target image.
    Initial = 3,
    /// Bootloader marks that the initial attempt at bootloading has started.
    Attempting = 2,
    /// Bootloader has encountered in `Attempting`, meaning that the application failed to `Confirm`.
    ///
    /// Or the application image did not pass verification and was never tried.
    Failed = 1,
    /// Application has marked the boot to be successful, and will boot it in the future.
    Confirmed = 0,
}

/// State record as stored in the State boot journal.
///
/// Care must be taken that `0xffff` is an invalid value,
/// as that is the typical value used by an empty NOR flash cell.
///
/// We ensure this by disallowing Slot value 0b111.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct State([u8; 2]);

impl State {
    pub const fn new(status: Status, target: Slot, backup: Slot) -> Self {
        let mut data = 0u8;
        data |= (status as u8) << 6;
        data |= backup.0 << 3;
        data |= target.0;

        let crc = CRC.checksum(&[data]);

        Self([data, crc])
    }

    pub fn try_new(data: [u8; 2]) -> Result<Self, ParseResult> {
        if data == [0xff, 0xff] {
            return Err(ParseResult::Unset);
        }

        if Self::try_target(data[0]).is_none() || Self::try_backup(data[0]).is_none() {
            return Err(ParseResult::Invalid);
        }

        if !Self::check_crc(data[1], data[0]) {
            return Err(ParseResult::Invalid);
        }

        Ok(State(data))
    }

    fn check_crc(crc: u8, data: u8) -> bool {
        crc == CRC.checksum(&[data])
    }

    pub fn status(&self) -> Status {
        // Note(unsafe): we are sure that any 2-bit u8 is a valid Status.
        unsafe { Status::try_from_primitive(self.0[0] >> 6).unwrap_unchecked() }
    }

    fn try_target(val: u8) -> Option<Slot> {
        Slot::try_from_u8(val & 0b111)
    }

    pub fn target(&self) -> Slot {
        // If Self exists, Slot must be valid.
        unsafe { State::try_target(self.0[0]).unwrap_unchecked() }
    }

    fn try_backup(val: u8) -> Option<Slot> {
        Slot::try_from_u8((val >> 3) & 0b111)
    }

    pub fn backup(&self) -> Slot {
        // If Self exists, Slot must be valid.
        unsafe { State::try_backup(self.0[0]).unwrap_unchecked() }
    }
}
