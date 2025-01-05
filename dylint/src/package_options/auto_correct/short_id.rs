use dylint_internal::git2::{Commit, Oid};

const SHORT_ID_LEN: usize = 7;

pub trait ShortId {
    fn short_id(&self) -> String;
}

impl ShortId for Commit<'_> {
    fn short_id(&self) -> String {
        self.id().short_id()
    }
}

impl ShortId for Oid {
    fn short_id(&self) -> String {
        self.to_string()[..SHORT_ID_LEN].to_owned()
    }
}
