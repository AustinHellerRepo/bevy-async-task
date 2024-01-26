use crate::{AsyncReceiver, AsyncTask, AsyncTaskStatus};
use bevy::{
    ecs::{
        component::Tick,
        system::{
            ExclusiveSystemParam, ReadOnlySystemParam, SystemMeta, SystemParam,
        },
        world::unsafe_world_cell::UnsafeWorldCell,
    },
    prelude::*,
    tasks::AsyncComputeTaskPool,
    utils::synccell::SyncCell,
};

/// A Bevy [`SystemParam`] to execute many similar [`AsyncTask`]s in the
/// background simultaneously.
pub struct AsyncTaskPool<'s, T>(
    pub(crate) &'s mut Vec<Option<AsyncReceiver<T>>>,
);

impl<'s, T> std::fmt::Debug for AsyncTaskPool<'s, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AsyncTaskPool {:p} (length: {})", self, self.0.len())
    }
}

impl<'s, T> AsyncTaskPool<'s, T> {
    /// Returns whether the task pool is idle.
    pub fn is_idle(&self) -> bool {
        self.0.is_empty() || !self.0.iter().any(Option::is_some)
    }

    /// Returns if empty or all tasks are finished.
    pub fn is_all_finished(&self) -> bool {
        for rx_option in self.0.iter() {
            if let Some(rx) = rx_option {
                if !rx.received {
                    dbg!("not received");
                    return false;
                }
                else {
                    dbg!("received");
                }
            }
            else {
                dbg!("rx_option is None");
                return false;
            }
        }
        dbg!("all finished");
        return true;
    }

    /// Spawn an async task in the background.
    pub fn spawn(&mut self, task: impl Into<AsyncTask<T>>) {
        let task = task.into();
        let (fut, rx) = task.into_parts();
        let task_pool = AsyncComputeTaskPool::get();
        let handle = task_pool.spawn(fut);
        handle.detach();
        self.0.push(Some(rx));
    }

    /// Iterate and poll the task pool for the current task statuses. A task can
    /// yield `Idle`, `Pending`, or `Finished(T)`.
    pub fn iter_poll(&mut self) -> impl Iterator<Item = AsyncTaskStatus<T>> {
        let mut statuses = vec![];
        self.0.retain_mut(|task| match task {
            Some(rx) => {
                if let Some(v) = rx.try_recv() {
                    statuses.push(AsyncTaskStatus::Finished(v));
                    false
                } else {
                    statuses.push(AsyncTaskStatus::Pending);
                    true
                }
            }
            None => {
                statuses.push(AsyncTaskStatus::Idle);
                true
            }
        });
        statuses.into_iter()
    }
}

impl<'_s, T: Send + 'static> ExclusiveSystemParam for AsyncTaskPool<'_s, T> {
    type State = SyncCell<Vec<Option<AsyncReceiver<T>>>>;
    type Item<'s> = AsyncTaskPool<'s, T>;

    fn init(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
        SyncCell::new(vec![])
    }

    #[inline]
    fn get_param<'s>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
    ) -> Self::Item<'s> {
        AsyncTaskPool(state.get())
    }
}

// SAFETY: only local state is accessed
unsafe impl<'s, T: Send + 'static> ReadOnlySystemParam
    for AsyncTaskPool<'s, T>
{
}

// SAFETY: only local state is accessed
unsafe impl<'a, T: Send + 'static> SystemParam for AsyncTaskPool<'a, T> {
    type State = SyncCell<Vec<Option<AsyncReceiver<T>>>>;
    type Item<'w, 's> = AsyncTaskPool<'s, T>;

    fn init_state(
        _world: &mut World,
        _system_meta: &mut SystemMeta,
    ) -> Self::State {
        SyncCell::new(vec![])
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        _system_meta: &SystemMeta,
        _world: UnsafeWorldCell<'w>,
        _change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        AsyncTaskPool(state.get())
    }
}
