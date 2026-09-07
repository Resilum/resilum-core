use rand_core::RngCore;

const OURS_BEGIN_WITH: char = 'R';
const TIEBREAK_LEN: usize = 3;
const ROOM_IN_THE_ADVERTISEMENT: usize = 8;
const CAN_HOST: u8 = 0b0000_0001;
const GROUP_IS_UP: u8 = 0b0000_0010;

pub const ON_THE_AIR_LEN: usize = TIEBREAK_LEN + 1;

pub const WE_TAKE_NO_NAME: &str = "";

pub type RandomEachAdvertisingSession = [u8; TIEBREAK_LEN];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Beacon {
    pub tiebreak: RandomEachAdvertisingSession,
    pub can_host: bool,
    pub group_is_up: bool,
}

impl Beacon {
    #[must_use]
    pub fn fresh(can_host: bool, group_is_up: bool) -> Self {
        let mut tiebreak = [0u8; TIEBREAK_LEN];
        rand_core::OsRng.fill_bytes(&mut tiebreak);
        Self {
            tiebreak,
            can_host,
            group_is_up,
        }
    }

    #[must_use]
    pub fn on_the_air(&self) -> [u8; ON_THE_AIR_LEN] {
        let mut raw = [0u8; ON_THE_AIR_LEN];
        raw[..TIEBREAK_LEN].copy_from_slice(&self.tiebreak);
        raw[TIEBREAK_LEN] =
            u8::from(self.can_host) * CAN_HOST + u8::from(self.group_is_up) * GROUP_IS_UP;
        raw
    }

    #[must_use]
    pub fn read_on_the_air(raw: &[u8]) -> Option<Self> {
        let (tiebreak, flags) = raw.split_at_checked(TIEBREAK_LEN)?;
        let flags = *flags.first()?;
        Some(Self {
            tiebreak: tiebreak.try_into().ok()?,
            can_host: flags & CAN_HOST != 0,
            group_is_up: flags & GROUP_IS_UP != 0,
        })
    }

    #[must_use]
    pub fn name(&self) -> String {
        let name = format!(
            "{OURS_BEGIN_WITH}{}",
            data_encoding::BASE32_NOPAD.encode(&self.on_the_air())
        );
        debug_assert!(name.len() <= ROOM_IN_THE_ADVERTISEMENT);
        name
    }

    #[must_use]
    pub fn read(name: &str) -> Option<Self> {
        let body = name.strip_prefix(OURS_BEGIN_WITH)?;
        let raw = data_encoding::BASE32_NOPAD.decode(body.as_bytes()).ok()?;
        Self::read_on_the_air(&raw)
    }

    #[must_use]
    pub fn heard(service_data: &[u8], name: Option<&str>) -> Option<Self> {
        Self::read_on_the_air(service_data).or_else(|| name.and_then(Self::read))
    }
}

#[cfg(test)]
mod tests;
