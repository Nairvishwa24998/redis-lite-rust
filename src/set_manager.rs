#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExpiryKind {
    Ex,
    Px,
    ExAt,
    PxAt,
}

pub struct SetLifecycle {
    nx: bool,
    xx: bool,
    get: bool,
    // Only adding expiry kind enum to match redis on allowing repeat of same expiry options without throwing error
    expiry_info: Option<(ExpiryKind, u64)>,
}

impl SetLifecycle {
    pub fn new() -> Self {
        SetLifecycle {
            nx: false,
            xx: false,
            get: false,
            expiry_info: None,
        }
    }

    pub fn get_nx(&self) -> bool {
        self.nx
    }

    pub fn set_nx(&mut self) {
        self.nx = true;
    }

    pub fn get_xx(&self) -> bool {
        self.xx
    }

    pub fn set_xx(&mut self) {
        self.xx = true;
    }

    pub fn get_get(&self) -> bool {
        self.get
    }

    pub fn set_get(&mut self) {
        self.get = true;
    }

    pub fn set_expiry_info(&mut self, expiry_kind: ExpiryKind, deadline: u64) {
        self.expiry_info = Some((expiry_kind, deadline));
    }

    pub fn get_expiry_info(&self) -> Option<(ExpiryKind, u64)> {
        self.expiry_info
    }
}
