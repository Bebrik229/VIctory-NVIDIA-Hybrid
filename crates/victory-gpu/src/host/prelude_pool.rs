use super::{Host, Prelude, host, lazy_prelude};
use std::sync::{Mutex, OnceLock};

const IDLE_LIMIT: usize = 128;

pub(super) struct Pool<T = Prelude>(Mutex<Vec<T>>);

// SAFETY: D3D12 objects are free-threaded. No list owns an idle prelude.
unsafe impl Send for Pool<Prelude> {}
// SAFETY: as above; the pairs are only reached through the lock.
unsafe impl Sync for Pool<Prelude> {}

pub(super) static IDLE: Pool = Pool(Mutex::new(Vec::new()));

fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("VICTORY_PRELUDE_POOL").is_some_and(|v| v == "1"))
}

fn make() -> Option<Prelude> {
    host().and_then(Host::make_prelude)
}

impl<T> Pool<T> {
    fn take(&self, make: impl FnOnce() -> Option<T>) -> Option<T> {
        let idle = self.0.lock().unwrap_or_else(|e| e.into_inner()).pop();
        if idle.is_some() {
            static SAID: std::sync::Once = std::sync::Once::new();
            SAID.call_once(|| tracing::info!("reused a state prelude (VICTORY_PRELUDE_POOL)"));
            idle
        } else {
            make()
        }
    }

    fn give(&self, prelude: T) {
        // Retaining the bounded set avoids recreating command memory on
        // every batch while host RAM pressure persists.
        let mut idle = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if idle.len() < IDLE_LIMIT {
            idle.push(prelude);
        }
    }
}

pub(super) enum Slot<T = Prelude> {
    Owned(OnceLock<Option<T>>),
    Pooled(Mutex<Option<T>>),
}

impl Slot {
    pub(super) fn new(owner: &Host) -> Self {
        let lazy = lazy_prelude();
        if lazy && enabled() {
            return Self::Pooled(Mutex::new(None));
        }
        let owned = OnceLock::new();
        if !lazy {
            let _ = owned.set(owner.make_prelude());
        }
        Self::Owned(owned)
    }

    pub(super) fn record<R>(&self, pool: &Pool, record: impl FnOnce(&Prelude) -> R) -> Option<R> {
        self.record_with(pool, make, record)
    }

}

impl<T> Slot<T> {
    fn record_with<R>(
        &self,
        pool: &Pool<T>,
        make: impl FnOnce() -> Option<T>,
        record: impl FnOnce(&T) -> R,
    ) -> Option<R> {
        match self {
            Self::Owned(owned) => owned.get_or_init(make).as_ref().map(record),
            Self::Pooled(held) => {
                let mut held = held.lock().unwrap_or_else(|e| e.into_inner());
                if held.is_none() {
                    *held = pool.take(make);
                }
                held.as_ref().map(record)
            }
        }
    }

    /// Called after the list's previous submission has finished and it has reset.
    pub(super) fn recycle(&self, pool: &Pool<T>) {
        if let Self::Pooled(held) = self {
            let prelude = held.lock().unwrap_or_else(|e| e.into_inner()).take();
            if let Some(prelude) = prelude {
                pool.give(prelude);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use windows::core::Interface;

    #[test]
    fn a_held_prelude_becomes_available_only_after_recycling() {
        if host().is_none() {
            return;
        }
        let pool = Pool(Mutex::new(Vec::new()));
        let first = Slot::Pooled(Mutex::new(None));
        let second = Slot::Pooled(Mutex::new(None));
        let allocator = |(allocator, _): &Prelude| allocator.as_raw() as usize;
        let held = first.record(&pool, allocator).unwrap();
        assert_eq!(first.record(&pool, allocator), Some(held));
        let other = second.record(&pool, allocator).unwrap();
        assert_ne!(other, held);
        first.recycle(&pool);
        let third = Slot::Pooled(Mutex::new(None));
        assert_eq!(third.record(&pool, allocator), Some(held));
    }

    fn pooled() -> Slot<u32> {
        Slot::Pooled(Mutex::new(None))
    }

    #[test]
    fn a_pooled_slot_shares_its_pair_only_after_recycling() {
        let pool = Pool::<u32>(Mutex::new(Vec::new()));
        let made = Cell::new(0u32);
        let make = || {
            made.set(made.get() + 1);
            Some(made.get())
        };
        let (first, second) = (pooled(), pooled());
        assert_eq!(first.record_with(&pool, make, |p| *p), Some(1));
        // Held until the list resets: a second submission records into the same pair.
        assert_eq!(first.record_with(&pool, make, |p| *p), Some(1));
        assert_eq!(second.record_with(&pool, make, |p| *p), Some(2));
        first.recycle(&pool);
        // Nothing held: a second reset gives nothing back twice.
        first.recycle(&pool);
        assert_eq!(pool.0.lock().unwrap().len(), 1);
        assert_eq!(pooled().record_with(&pool, make, |p| *p), Some(1));
        assert_eq!(first.record_with(&pool, make, |p| *p), Some(3));
        assert_eq!(made.get(), 3);
    }

    #[test]
    fn an_owned_slot_keeps_its_pair_through_a_reset() {
        let pool = Pool::<u32>(Mutex::new(Vec::new()));
        let slot = Slot::Owned(OnceLock::new());
        assert_eq!(slot.record_with(&pool, || Some(7), |p| *p), Some(7));
        slot.recycle(&pool);
        assert!(pool.0.lock().unwrap().is_empty());
        assert_eq!(slot.record_with(&pool, || Some(8), |p| *p), Some(7));
    }

    #[test]
    fn a_pair_that_cannot_be_made_records_nothing() {
        let pool = Pool::<u32>(Mutex::new(Vec::new()));
        assert_eq!(pooled().record_with(&pool, || None, |p| *p), None);
    }

    #[test]
    fn the_idle_pool_keeps_a_fixed_number_of_pairs() {
        let pool = Pool::<u32>(Mutex::new(Vec::new()));
        for i in 0..IDLE_LIMIT as u32 + 5 {
            pool.give(i);
        }
        assert_eq!(pool.0.lock().unwrap().len(), IDLE_LIMIT);
        // What was kept is handed out again without making a pair.
        for _ in 0..IDLE_LIMIT {
            assert!(pool.take(|| None).is_some());
        }
        assert_eq!(pool.take(|| None), None);
    }
}
