use crate::E2EStatus;

pub trait CounterOps {
    type CounterType;
    const MAX_VALUE: Self::CounterType;
    const MODULO: u64;

    fn increment_counter(current: Self::CounterType) -> Self::CounterType;
    fn check_counter_delta(
        current: Self::CounterType,
        received: Self::CounterType,
    ) -> Self::CounterType;
    fn validate_counter(
        current: Self::CounterType,
        received: Self::CounterType,
        max_delta: Self::CounterType,
        initialized: bool,
    ) -> E2EStatus;
}

pub struct Counter8;
pub struct Counter16;
pub struct Counter32;

/// Specialized 4-bit counter for Profile 11 (0-14, modulo 15)
pub struct Counter4Profile11;

/// Specialized 4-bit counter for Profile 22 (0-15, modulo 16)
pub struct Counter4Profile22;

impl CounterOps for Counter8 {
    type CounterType = u8;
    const MAX_VALUE: u8 = 0xFF;
    const MODULO: u64 = 0x100;

    fn increment_counter(current: u8) -> u8 {
        (current as u16 + 1) as u8 & Self::MAX_VALUE
    }

    fn check_counter_delta(current: u8, received: u8) -> u8 {
        if received >= current {
            received - current
        } else {
            ((Self::MODULO + received as u64 - current as u64) % Self::MODULO) as u8
        }
    }

    fn validate_counter(current: u8, received: u8, max_delta: u8, initialized: bool) -> E2EStatus {
        let delta = Self::check_counter_delta(current, received);

        if delta == 0 {
            if initialized {
                E2EStatus::Repeated
            } else {
                E2EStatus::Ok
            }
        } else if delta == 1 {
            E2EStatus::Ok
        } else if delta >= 2 && delta <= max_delta {
            E2EStatus::OkSomeLost
        } else {
            E2EStatus::WrongSequence
        }
    }
}

impl CounterOps for Counter16 {
    type CounterType = u16;
    const MAX_VALUE: u16 = 0xFFFF;
    const MODULO: u64 = 0x10000;

    fn increment_counter(current: u16) -> u16 {
        (current as u32 + 1) as u16 & Self::MAX_VALUE
    }

    fn check_counter_delta(current: u16, received: u16) -> u16 {
        if received >= current {
            received - current
        } else {
            ((Self::MODULO + received as u64 - current as u64) % Self::MODULO) as u16
        }
    }

    fn validate_counter(
        current: u16,
        received: u16,
        max_delta: u16,
        initialized: bool,
    ) -> E2EStatus {
        let delta = Self::check_counter_delta(current, received);

        if delta == 0 {
            if initialized {
                E2EStatus::Repeated
            } else {
                E2EStatus::Ok
            }
        } else if delta == 1 {
            E2EStatus::Ok
        } else if delta >= 2 && delta <= max_delta {
            E2EStatus::OkSomeLost
        } else {
            E2EStatus::WrongSequence
        }
    }
}

impl CounterOps for Counter32 {
    type CounterType = u32;
    const MAX_VALUE: u32 = 0xFFFFFFFF;
    const MODULO: u64 = 0x100000000;

    fn increment_counter(current: u32) -> u32 {
        if current == Self::MAX_VALUE {
            0x00000000
        } else {
            (current + 1) & Self::MAX_VALUE
        }
    }

    fn check_counter_delta(current: u32, received: u32) -> u32 {
        if received >= current {
            received - current
        } else {
            ((Self::MODULO + received as u64 - current as u64) % Self::MODULO) as u32
        }
    }

    fn validate_counter(
        current: u32,
        received: u32,
        max_delta: u32,
        initialized: bool,
    ) -> E2EStatus {
        let delta = Self::check_counter_delta(current, received);

        if delta == 0 {
            if initialized {
                E2EStatus::Repeated
            } else {
                E2EStatus::Ok
            }
        } else if delta == 1 {
            E2EStatus::Ok
        } else if delta >= 2 && delta <= max_delta {
            E2EStatus::OkSomeLost
        } else {
            E2EStatus::WrongSequence
        }
    }
}

impl CounterOps for Counter4Profile11 {
    type CounterType = u8;
    const MAX_VALUE: u8 = 14; // Profile 11 uses 0-14
    const MODULO: u64 = 15;

    fn increment_counter(current: u8) -> u8 {
        (current + 1) % 15
    }

    fn check_counter_delta(current: u8, received: u8) -> u8 {
        if received >= current {
            received - current
        } else {
            ((Self::MODULO + received as u64 - current as u64) % Self::MODULO) as u8
        }
    }

    fn validate_counter(current: u8, received: u8, max_delta: u8, initialized: bool) -> E2EStatus {
        let delta = Self::check_counter_delta(current, received);

        if delta == 0 {
            if initialized {
                E2EStatus::Repeated
            } else {
                E2EStatus::Ok
            }
        } else if delta == 1 {
            E2EStatus::Ok
        } else if delta >= 2 && delta <= max_delta {
            E2EStatus::OkSomeLost
        } else {
            E2EStatus::WrongSequence
        }
    }
}

impl CounterOps for Counter4Profile22 {
    type CounterType = u8;
    const MAX_VALUE: u8 = 15; // Profile 22 uses 0-15
    const MODULO: u64 = 16;

    fn increment_counter(current: u8) -> u8 {
        (current + 1) % 16
    }

    fn check_counter_delta(current: u8, received: u8) -> u8 {
        if received >= current {
            received - current
        } else {
            ((Self::MODULO + received as u64 - current as u64) % Self::MODULO) as u8
        }
    }

    fn validate_counter(current: u8, received: u8, max_delta: u8, _initialized: bool) -> E2EStatus {
        let delta = Self::check_counter_delta(current, received);

        if delta == 0 {
            E2EStatus::Repeated
        } else if delta == 1 {
            E2EStatus::Ok
        } else if delta >= 2 && delta <= max_delta {
            E2EStatus::OkSomeLost
        } else {
            E2EStatus::WrongSequence
        }
    }
}
